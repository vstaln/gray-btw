#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/gray-btw.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/gray-btw" ;;
    *) rel="x86_64-apple-darwin/gray-btw" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/gray-btw" ;;
    *) rel="x86_64-unknown-linux-musl/gray-btw" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/gray-btw.exe" "$here/target/release/gray-btw" "$here/target/debug/gray-btw.exe" "$here/target/debug/gray-btw"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "gray-btw binary not found in $here" >&2
exit 1
