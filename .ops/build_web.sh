#!/bin/sh
#
# Usage: ./ops/build_web.sh

# Build WASM binaries.
cargo build -p codas-cdylib --release --target=wasm32-unknown-unknown --features=wasm

# Generate WASM-JS shims.
cargo install -q --root target/ --version 0.2.100 wasm-bindgen-cli
./target/bin/wasm-bindgen --target no-modules --out-dir target/web/ target/wasm32-unknown-unknown/release/codas.wasm --no-typescript

# Rename WASM artifact to match JS.
mv target/web/codas_bg.wasm target/web/codas.wasm