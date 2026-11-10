#!/usr/bin/env bash
# Builds the browser prototype: backend rules -> WebAssembly -> one self-contained HTML file.
# Needs: Rust with the wasm32-unknown-unknown target, and python3.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown \
  || rustup target add wasm32-unknown-unknown

cargo build --release --target wasm32-unknown-unknown --manifest-path "$here/wasm/Cargo.toml"

wasm="$here/wasm/target/wasm32-unknown-unknown/release/phoenix_wasm.wasm"
mkdir -p "$here/web/dist"
python3 - "$wasm" "$here/web/index.template.html" "$here/web/dist/index.html" <<'PY'
import base64, sys
wasm, template, out = sys.argv[1:]
b64 = base64.b64encode(open(wasm, "rb").read()).decode()
page = open(template, encoding="utf-8").read().replace("__WASM_B64__", b64)
head = ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">'
        '</head><body style="margin:0">')
open(out, "w", encoding="utf-8").write(head + page + "</body></html>")
PY
echo "Built $here/web/dist/index.html ($(du -k "$here/web/dist/index.html" | cut -f1) KB). Open it in a browser."
