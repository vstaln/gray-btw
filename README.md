<div align="center">
  <img alt="gray-btw" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-btw</h1>
  <p><strong>Side conversations inside a session, without hijacking the main thread.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://gray.alignment.id/plugins/gray-btw">Store</a> ·
    <a href="https://github.com/vstaln/gray-btw">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <a href="https://gray.alignment.id/plugins/gray-btw"><img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" /></a>
  </p>
</div>

<br/>

```bash
gray plugin install gray-btw
```

Side conversations inside a session. `/btw <text>` asks a question on a
separate channel via `host/run` instead of hijacking the main thread; the
answer comes back as the command output and the exchange is appended to a
per-session thread log.

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

## Tags

`gray` `plugin` `btw` `rust`

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
