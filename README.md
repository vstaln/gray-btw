# gray-btw

Side conversations inside a session. `/btw <text>` asks a question on a
separate channel via `host/run` instead of hijacking the main thread; the
answer comes back as the command output and the exchange is appended to a
per-session thread log. Port of [pi-btw](https://www.npmjs.com/package/pi-btw)
(MIT; the side-conversation subset — no overlay UI).

## Commands

- `/btw <text>` — ask a side question (goes out as
  `Side conversation (btw) — answer briefly, no tools unless essential: <text>`)
- `/btw log` — render the numbered thread
- `/btw <N>` — repeat the Nth exchange's question
- `/btw clear` — wipe the thread

Each exchange is one line in `~/.gray/btw/<sid>.jsonl`:
`{"q": "…", "a": "…", "ts": …}`.

Without the `host.turn` capability, `/btw` degrades to a hint instead of
failing. Turns are sequential — a non-blocking feel is aspirational.

## Wire

- claims: `command/run` (`/btw`), `plugin/manifest`, `plugin/shutdown`
- sidecar→host: `host/run` (blocking, 40s TTL; capability `host.turn`)
- protocol `1.1`, no tools, no hooks

## Install

```
gray plugin install ~/grayplugins/gray-btw
gray plugin capabilities btw --all
```
