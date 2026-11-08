# Optional authored Valley Farm angle memory

The authenticated authored Valley solo Farm remembers the last observed Ctrl or Alt+Ctrl rotation for up to 30 seconds. Reopening the Farm tool restores the angle through the existing rotation and position validation. A new match resets the temporary angle records. Paid Farm positions and rotations use the existing save format.

In a fresh optional source copy, apply the existing Farm cursor patch followed by the angle memory patch.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley74-city-footprint
```

For local authored Valley use, the generated WASM is served as `docs/openepoch-ee2-valley-preview.wasm`. The expected SHA-256 is `0c959edf156ad828536b999ac039bb26883569c634ee80949a177706d8e527a5`. The angle patch SHA-256 is `1a45ff520653f02ef612edb16bb064dccb6979bb87b381fcb2632538bcaad593`. The locally installed map and assets are separate inputs.

Build standard skirmish from the base source tree with `cargo build --offline --release --target wasm32-unknown-unknown --features ee2-native13`. The standard SHA-256 is `7cbdf696b946e0b6a9f74c04cf41b2f685c75e8f21188993e3ffc04cd4e16fb9`.
