<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-btw</h1>
<p align="center">Side conversations inside a session — ask without hijacking the main thread.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-btw/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

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

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
