# Optional Huge10 automatic Dock placement

Apply this source patch to the private Huge10 Resume131 source variant. The standard and authored Valley carriers use separate sources.

The base private carrier SHA-256 is `7f31f7812e084a390a875d39d3a9ea7afb654254d7d3e96c66f18a8565f6393a`. Before applying, verify the source hashes below.

| Source path | Base SHA-256 |
| --- | --- |
| `Cargo.toml` | `8ebac5bc5cf7cf8d729592ed8eb543614b0faf1e028b954e0f3ca5c04f94360c` |
| `src/cmd.rs` | `c69af3b695c1aae5b084169c32809c5403ecc81b3555cc1eca32b80024f42507` |
| `src/dock157.rs` | Absent |
| `src/lib.rs` | `3c223e0c52c66dc45a6bc1811f084200082125ad72733da3f1bfc716027f4f03` |
| `src/placement.rs` | `6d565a1cd4b6a6c055b6ddb7c3dc5c8daf259e8e3be59fcd73a8258016f1245c` |
| `src/render.rs` | `db8e0e5c69d3ea34b211fd12bb70748917b3fe03ef156e147dc9dc77fe8c72b3` |

From the private source root, run these commands with the patch available locally.

```sh
git -c core.autocrlf=false apply --whitespace=nowarn /path/to/dock-shore157-engine.patch
cargo build --offline --release --target wasm32-unknown-unknown --features ee2-pangea-huge10-dock157
```

Keep the existing private geometry beside the private source root. Supply the separately generated shore input at `../private-geometry/huge10-dock-shore.bin`.
Its SHA-256 is `33a9af433f1fe930afd84cdf0bdae749747cadf2d43ef8294afd48ef144828fc`. The optional source test fixture is also private.
The resulting private Dock157 WASM SHA-256 is `23f8940f77ddf66952710cfacf52ce84eccdfdbab104c175316b2ff4e0ae5e15`. The released local and Mac runtime embeds the shore input in that WASM.
The original archive, generated geometry and private entry point stay in the private carrier.
