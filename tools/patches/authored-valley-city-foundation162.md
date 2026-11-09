# Optional authored Valley carrier for Native127

The standard Native127 source includes generated solo City Center foundations. Build the standard WASM directly with the `ee2-native13` feature. Start the optional build from a fresh copy of the same source and apply the four patches below in order.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-house-cursor155.patch
git -c core.autocrlf=false apply --check --whitespace=nowarn tools/patches/authored-valley-city-foundation162-over-generated163.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-city-foundation162-over-generated163.patch
CARGO_BUILD_JOBS=1 cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley74-city-footprint
```

Use rustc 1.99.1 to reproduce the observed binary hashes. The standard WASM is `718bc81a7631774e448396bf52930207b787da7f00f735cb236e803bcea1a366`. The optional Valley WASM is `42e6b03636b81d8614be6778ef4657348a86791205a3d34006a0dd2e66e1982d`. The rebased optional patch has SHA-256 `637be56fae0f34c23bda1d485b88c0a6b3828534b56a3e10629d44218c33c241`. The optional compiled WASM and authenticated original assets belong to the local and Mac deliveries. For historical Native126 reproduction, use its published source and original City162 recipe.

Fresh generated UP solo games opt in during normal setup and record their rule in a checksummed `CSH1` save envelope. Paid City Center sites start with one HP, with ordinary damage enabled by first worker labor. Existing generated and Valley archives keep their saved rules. Earlier engines reject new `CSH1` saves, which can be retained for the compatible carrier.

The optional carrier preserves authenticated Valley V37S 1–12 and prefers `valley77_commit` for new authenticated games. The web menu accepts the earlier compatible `valley76_commit` when using an older optional carrier. Multiplayer keeps the established construction behavior and lockstep protocol. The original one-HP rebuild is documented in an owned region. Exact original first acquisition timing for an unowned region remains a separate question.
