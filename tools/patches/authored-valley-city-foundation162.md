# Optional authored Valley carrier for Native128

Start with a fresh copy of the published Native128 source. The standard carrier uses `ee2-native13`. The optional authenticated Valley carrier applies the following media-free patches in order.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-house-cursor155.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-city-foundation162-over-generated163.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/forest-order164-over-generated163.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/forest-clock166.patch
CARGO_BUILD_JOBS=1 cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley166-forest-clock
```

The standard WASM SHA-256 is `718bc81a7631774e448396bf52930207b787da7f00f735cb236e803bcea1a366`. The optional WASM SHA-256 is `12b35944434a6dcdb581092a1fab2e4d2881bfe900cd59d0066c8bc38a6384dd`. The optional source recipe was replayed against all 219 Cargo and Rust files in the sealed tree. The compiled optional carrier, authenticated maps and original stump graphics belong to the local and Mac deliveries.

Generated solo `CSH1` archives and older authored Valley archives retain their saved rules. Playable forest state is stored in `V37S13`. The first observed chop or fall event upgrades its untimed sidecar from `VFP1` to `VFP2` with source-clock words. A Native127 rollback can continue older archives and generated `CSH1`. Keep newer forest archives for a compatible private carrier.
