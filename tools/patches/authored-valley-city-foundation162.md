# Optional authored Valley carrier for Native129

Start with a fresh copy of the published Native129 source. The standard carrier uses `ee2-native13`. The optional authenticated Valley carrier applies the following media-free patches in order.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm147.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-farm-angle148.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-house-cursor155.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/authored-valley-city-foundation162-over-generated163.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/forest-order164-over-generated163.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/forest-clock166.patch
git -c core.autocrlf=false apply --whitespace=nowarn tools/patches/forest-retarget169.patch
CARGO_BUILD_JOBS=1 cargo build --offline --release --target wasm32-unknown-unknown --features ee2-valley169-forest-retarget
```

The standard WASM SHA-256 is `718bc81a7631774e448396bf52930207b787da7f00f735cb236e803bcea1a366`. The optional WASM SHA-256 is `d08d241729b25e65ce7f583335e786a61cb661a9b92e5dac118a02141281da6b`. The optional recipe was replayed against all 219 Cargo and Rust files of the candidate source. The optional compiled carrier and original assets are in the local and Mac deliveries.

Authenticated local Valley workers can move to another forest actor when a tree runs out of wood, carrying any partial load with them. The worker, order, cargo and forest data keep their existing `V37S13` and `VFP1`/`VFP2` formats. Older playable Valley archives load byte-for-byte and use the new target selection after depletion when played on this carrier. Native128 can read the saved orders after a replacement is assigned and continues with its previous selection policy. Ordinary generated skirmishes, network matches and the standard carrier use their existing gather logic.
