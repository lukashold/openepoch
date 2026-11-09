#!/usr/bin/env sh
# Rebuild all three independent numerical profiles. Playing needs only Python.
set -eu
cd "$(dirname "$0")"
cargo build --offline --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch.wasm
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-reference
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch-ee2.wasm
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-ordered-claims217
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch-ee2-up15.wasm
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch-ee2-roster6.wasm
cp target/wasm32-unknown-unknown/release/openepoch.wasm docs/openepoch-citizen9.wasm
python3 tools/build_source_cloth.py
