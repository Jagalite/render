#!/bin/sh
set -eu
cargo build --locked --release -p render-web --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/render_web.wasm
# Only interoperability bootstrap is generated here; application behavior lives in Rust.
printf '%s\n' '// Generated interop bootstrap. Rust owns application behavior.' 'import init from "./render_web.js";' 'await init();' > web/pkg/bootstrap.js
