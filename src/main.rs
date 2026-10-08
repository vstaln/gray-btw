//! gray-btw — side conversations inside a session.
//!
//! Port of pi-btw (MIT, subset): `/btw <text>` asks a question on a separate
//! side channel instead of hijacking the main thread. It goes through
//! `host/run` (capability `host.turn`) with a "side conversation" prompt;
//! the answer is the command's `{text}` reply and the exchange is appended
//! to `~/.gray/btw/<sid>.jsonl`.
//!
//!   /btw <text>  ask a side question
//!   /btw log     render the thread
//!   /btw <N>     repeat the Nth exchange's question
//!   /btw clear   wipe the thread log
//!
//! When `host.turn` isn't granted the command degrades to a hint instead of
//! crashing. Turns are sequential — non-blocking UI is aspirational.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

type Pending = Arc<Mutex<HashMap<String, mpsc::Sender<Value>>>>;
const HOST_RUN_TTL: Duration = Duration::from_secs(40);
const NEED_GRANT: &str =
    "btw needs the host.turn capability: gray plugin capabilities btw --all";

// ---------- state -------------------------------------------------------------

fn gray_home() -> PathBuf {
    std::env::var_os("GRAY_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn sanitize(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '-' })
        .collect();
    if s.is_empty() { "session".into() } else { s }
}

fn log_dir() -> PathBuf {
    gray_home().join("btw")
}

fn log_path(sid: &str) -> PathBuf {
    log_dir().join(format!("{}.jsonl", sanitize(sid)))
}

fn session_id(params: &Value) -> String {
    params
        .get("session")
        .and_then(|s| s.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn append_log_in(dir: &Path, sid: &str, q: &str, a: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = json!({"q": q, "a": a, "ts": ts}).to_string();
    let path = dir.join(format!("{}.jsonl", sanitize(sid)));
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{line}").map_err(|e| e.to_string())
}

/// The btw thread log: (question, answer) pairs in order.
fn read_log_in(dir: &Path, sid: &str) -> Vec<(String, String)> {
    let path = dir.join(format!("{}.jsonl", sanitize(sid)));
    let Ok(raw) = std::fs::read_to_string(path) else { return Vec::new() };
    raw.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| {
            Some((
                v.get("q").and_then(Value::as_str)?.to_string(),
                v.get("a").and_then(Value::as_str)?.to_string(),
            ))
        })
        .collect()
}

fn render_log(thread: &[(String, String)]) -> String {
    if thread.is_empty() {
        return "no btw thread yet — /btw <question> starts one".into();
    }
    let mut out = String::new();
    for (i, (q, a)) in thread.iter().enumerate() {
        out.push_str(&format!("{}. Q: {}\n   A: {}\n", i + 1, q, a));
    }
    out.trim_end().to_string()
}

// ---------- host/run -----------------------------------------------------------

struct Shared {
    out: Arc<Mutex<std::io::Stdout>>,
    pending: Pending,
    counter: Arc<Mutex<u64>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            out: Arc::new(Mutex::new(std::io::stdout())),
            pending: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
        }
    }
}

fn next_id(counter: &Arc<Mutex<u64>>) -> String {
    let mut n = counter.lock().expect("counter");
    *n += 1;
    format!("q{n}")
}

fn host_send(out: &Arc<Mutex<std::io::Stdout>>, method: &str, params: Value, id: &str) {
    let req = json!({"id": id, "method": method, "params": params});
    let mut o = out.lock().expect("stdout");
    let _ = writeln!(o, "{req}");
    let _ = o.flush();
}

/// Blocking host/run — the side conversation itself. The whole reply value
/// (result or error) comes back on the pending channel.
fn host_run(sh: &Shared, prompt: &str) -> Result<String, String> {
    let id = next_id(&sh.counter);
    let (tx, rx) = mpsc::channel();
    sh.pending.lock().expect("pending").insert(id.clone(), tx);
    host_send(&sh.out, "host/run", json!({"prompt": prompt}), &id);
    let res = rx.recv_timeout(HOST_RUN_TTL);
    sh.pending.lock().expect("pending").remove(&id);
    match res {
        Err(_) => Err(NEED_GRANT.into()),
        Ok(v) => {
            if let Some(e) = v.pointer("/error/message").and_then(Value::as_str) {
                return Err(format!("{e} — {NEED_GRANT}"));
            }
            let text = v
                .pointer("/result/text")
                .or_else(|| v.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() {
                Err(format!("host/run returned nothing — {NEED_GRANT}"))
            } else {
                Ok(text)
            }
        }
    }
}

fn ask_btw(sh: &Shared, question: &str) -> Result<String, String> {
    host_run(
        sh,
        &format!(
            "Side conversation (btw) — answer briefly, no tools unless essential: {question}"
        ),
    )
}

// ---------- /btw ----------------------------------------------------------------

/// Parsed `/btw` argv — separated from effects so it stays testable.
enum BtwCmd {
    Ask(String),
    Log,
    Clear,
    Repeat(usize),
    Help,
}

fn parse_btw(argv: &[&str], thread_len: usize) -> BtwCmd {
    match argv.first().copied() {
        None => BtwCmd::Help,
        Some("log") => BtwCmd::Log,
        Some("clear") | Some("reset") | Some("new") => BtwCmd::Clear,
        Some("help") => BtwCmd::Help,
        Some(first) if argv.len() == 1 => {
            match first.parse::<usize>() {
                Ok(n) if n >= 1 && n <= thread_len => BtwCmd::Repeat(n),
                _ => BtwCmd::Ask(first.to_string()),
            }
        }
        _ => BtwCmd::Ask(argv.join(" ")),
    }
}

fn run_command(sh: &Shared, params: &Value, argv: &[&str]) -> String {
    let sid = session_id(params);
    let thread = read_log_in(&log_dir(), &sid);
    match parse_btw(argv, thread.len()) {
        BtwCmd::Help => format!(
            "gray-btw {} — side conversations.\n/btw <text> ask · /btw log · /btw <N> repeat the Nth question · /btw clear",
            env!("CARGO_PKG_VERSION")
        ),
        BtwCmd::Log => render_log(&thread),
        BtwCmd::Clear => {
            let p = log_path(&sid);
            match std::fs::remove_file(&p) {
                Ok(()) => "btw thread cleared".into(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "no btw thread to clear".into(),
                Err(e) => format!("couldn't clear {}: {e}", p.display()),
            }
        }
        BtwCmd::Repeat(n) => {
            let q = thread[n - 1].0.clone();
            match ask_btw(sh, &q) {
                Ok(a) => {
                    let _ = append_log_in(&log_dir(), &sid, &q, &a);
                    a
                }
                Err(e) => e,
            }
        }
        BtwCmd::Ask(q) => match ask_btw(sh, &q) {
            Ok(a) => {
                if let Err(e) = append_log_in(&log_dir(), &sid, &q, &a) {
                    format!("{a}\n\n(couldn't log to btw thread: {e})")
                } else {
                    a
                }
            }
            Err(e) => e,
        },
    }
}

// ---------- wire ---------------------------------------------------------------

fn manifest() -> Value {
    json!({
        "name": "btw",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "1.1",
        "capabilities": ["host.turn"],
        "tools": [],
        "commands": ["/btw"],
    })
}

/// One request → `Some(reply)`, or `None` for notifications. The bool asks
/// the loop to exit after writing the reply.
fn handle(sh: &Shared, req: &Value) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        return (None, method == "plugin/shutdown");
    };
    let result = match method {
        "plugin/manifest" => manifest(),
        "command/run" => {
            let argv: Vec<&str> = params
                .get("argv")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            json!({ "text": run_command(sh, &params, &argv) })
        }
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("manifest") {
        println!("{}", manifest());
        return;
    }
    let sh = Shared::new();
    let pending = sh.pending.clone();
    let out = sh.out.clone();

    // Reader thread: host→sidecar `host/*` replies arrive with a string id
    // and no method; route the whole value to whoever is waiting. Numeric-id
    // requests go to the work loop.
    let (work_tx, work_rx) = mpsc::channel::<Value>();
    let _reader = std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
            if v.get("method").is_none()
                && let Some(id) = v.get("id").and_then(Value::as_str)
            {
                if let Some(tx) = pending.lock().expect("pending").remove(id) {
                    let _ = tx.send(v);
                }
                continue;
            }
            if work_tx.send(v).is_err() {
                break;
            }
        }
    });

    for req in work_rx {
        let (reply, exit) = handle(&sh, &req);
        if let Some(reply) = reply {
            let mut o = out.lock().expect("stdout");
            let _ = writeln!(o, "{reply}");
            let _ = o.flush();
        }
        if exit {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> Shared {
        Shared::new()
    }

    fn call(sh: &Shared, method: &str, params: Value) -> Value {
        handle(sh, &json!({ "id": 1, "method": method, "params": params })).0.unwrap()
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gray-btw-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn manifest_shape() {
        let m = call(&shared(), "plugin/manifest", Value::Null)["result"].clone();
        assert_eq!(m["name"], "btw");
        assert_eq!(m["protocol"], "1.1");
        assert_eq!(m["capabilities"], json!(["host.turn"]));
        assert_eq!(m["commands"], json!(["/btw"]));
        assert_eq!(m["tools"], json!([]));
    }

    #[test]
    fn log_roundtrip() {
        let d = tmp_dir("rt");
        append_log_in(&d, "s1", "why?", "because").unwrap();
        append_log_in(&d, "s1", "and then?", "done").unwrap();
        let t = read_log_in(&d, "s1");
        assert_eq!(t, vec![("why?".to_string(), "because".to_string()), ("and then?".to_string(), "done".to_string())]);
        assert!(read_log_in(&d, "other").is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn render_numbers_exchanges() {
        let t = vec![("q1".to_string(), "a1".to_string()), ("q2".to_string(), "a2".to_string())];
        let r = render_log(&t);
        assert!(r.contains("1. Q: q1") && r.contains("2. Q: q2"), "{r}");
        assert!(render_log(&[]).contains("no btw thread"));
    }

    #[test]
    fn parse_routes_argv() {
        assert!(matches!(parse_btw(&[], 3), BtwCmd::Help));
        assert!(matches!(parse_btw(&["log"], 3), BtwCmd::Log));
        assert!(matches!(parse_btw(&["clear"], 3), BtwCmd::Clear));
        assert!(matches!(parse_btw(&["2"], 3), BtwCmd::Repeat(2)));
        // out of range / extra words → treated as a question
        assert!(matches!(parse_btw(&["9"], 3), BtwCmd::Ask(_)));
        match parse_btw(&["what", "is", "2+2?"], 3) {
            BtwCmd::Ask(q) => assert_eq!(q, "what is 2+2?"),
            _ => panic!("expected Ask"),
        }
    }

    #[test]
    fn bare_btw_prints_help() {
        let r = call(&shared(), "command/run", json!({"name":"/btw","argv":[],"session":{"id":"s"}}));
        assert!(r["result"]["text"].as_str().unwrap().contains("/btw <text>"));
    }

    #[test]
    fn btw_log_uses_session_log() {
        let d = tmp_dir("cmd");
        append_log_in(&d, "s1", "q", "a").unwrap();
        // render directly — the command path is env-dependent; logic shared
        assert!(render_log(&read_log_in(&d, "s1")).contains("Q: q"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sanitize_and_shutdown() {
        assert_eq!(sanitize("a/b"), "a-b");
        let (reply, exit) = handle(&shared(), &json!({"id": 2, "method": "plugin/shutdown"}));
        assert!(reply.is_some() && exit);
    }
}
