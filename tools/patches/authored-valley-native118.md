# Optional authored Valley carrier for Native118

Apply these patches in a fresh copy of the standard Native118 source.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-house-cursor155.patch
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley74-city-footprint
```

The authored Valley carrier SHA-256 is `5b3b68d9ab3a1ce4ac8ed0f75b6a320eead648168657f833884cfbb7c0f02a6b`. The original map and assets are separate local files.
The standard release uses `ee2-native13` and has SHA-256 `62bbdd30de55ea8f21be7f1bb4904926ff50b3ac244f6294b605c269a30b864e`.
