# Optional Valley Farm cursor

This source patch builds the optional authored Valley carrier. Use a separate copy of the Native108 source tree and run the commands from its root.

```sh
git -c core.autocrlf=false apply --check tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley74-city-footprint
```

The optional build produces `target/wasm32-unknown-unknown/release/openepoch.wasm`. In a local authored Valley installation it is served as `docs/openepoch-ee2-valley-preview.wasm`. The authenticated local map and media are separate inputs.

Rust and Cargo 1.99.1 were used for the verified build. The optional WASM SHA-256 is `24edc3bedf3f48bafa84c32d5f8b4537ec5aa0ea28dccbbb96f35825402bb23c`. The Rust patch SHA-256 is `797e9ef0a468108660648080006a3c92ee7fd11e48f7437f00ce2677de524a66`.

Build the standard edition from the base source tree with `cargo build --offline --release --target wasm32-unknown-unknown --features ee2-native13`. Its verified WASM SHA-256 is `7cbdf696b946e0b6a9f74c04cf41b2f685c75e8f21188993e3ffc04cd4e16fb9`.

In the authored Valley solo game, preview and paid placement share the Farm pose. Ctrl rotates by 45 degrees and Alt+Ctrl by approximately 11.25 degrees. The candidate checks use the recovered original static terrain rules and alternate positions. The existing OpenEpoch authority handles territory, collisions and worker routing. Remembered angles and per-user patch settings require further work.
