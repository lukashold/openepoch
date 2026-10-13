#!/usr/bin/env sh
# Build the simulation to WebAssembly and drop it next to the page.
set -e
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch.wasm
echo "Built docs/openepoch.wasm - serve the docs/ folder, e.g.: python -m http.server --directory docs 8123"
