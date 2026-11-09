# Optional authored Valley House cursor

Apply these patches in a fresh optional source copy after the standard Native116 source is staged.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-house-cursor155.patch
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley74-city-footprint
```

The local authored carrier is `docs/openepoch-ee2-valley-preview.wasm` with SHA-256 `ad0053578c6b4adb1eb956977632fdbfefa455a2e7d929f554eb206af75d731e`.
The map and original media are separate local assets.

The standard source build uses `ee2-native13` and yields `e46ddc0a962304075ec8972fc6f6d6dfa44909b89c9f4c2768d559f9d54a30df`.
