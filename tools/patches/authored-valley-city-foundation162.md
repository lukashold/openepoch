# Optional authored Valley carrier for Native130

Start with a fresh copy of the published Native130 source. The standard carrier uses `ee2-native13`. The optional authenticated Valley carrier applies the following media-free patches in order.

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

The standard WASM SHA-256 is `220a4bff4a5768c1f270a539cf25e0df67198678ef4d1c49994825562c82eea6`. The optional WASM SHA-256 is `61758cf092ab1a73c34e7e2097c2b63740726f2c6f4d38d0035c23451f535b64`. The optional recipe was replayed against all 219 Cargo and Rust files of the candidate source. The optional compiled carrier and original assets are in the local and Mac deliveries.

Authenticated local Valley workers can move to another forest actor when a tree runs out of wood, carrying any partial load with them. The worker, order, cargo and forest data keep their existing `V37S13` and `VFP1`/`VFP2` formats. Older playable Valley archives load byte-for-byte and use the new target selection after depletion when played on this carrier. Native128 can read the saved orders after a replacement is assigned and continues with its previous selection policy. Both UP carriers use the shared resource epoch windows and the same market retirement rules.
