# Optional Huge10 Dock cursor updates

Apply the Dock157 engine patch first to the private Huge10 Resume131 source variant. This follow-up preserves the standard and authored Valley engines.

The authenticated local/Mac Dock157 carrier SHA-256 is `23f8940f77ddf66952710cfacf52ce84eccdfdbab104c175316b2ff4e0ae5e15`. Verify the exact Dock157 source before applying this patch.

| Source path | Dock157 SHA-256 | DockCursor158 SHA-256 |
| --- | --- | --- |
| `src/cmd.rs` | `1509b229572f909383f1b18bd520692839b67b8c4fa8729ffa00239830d04e44` | `fbcbc77b46d8c03bb5502f35457cb1cfa8ffff17d04abeb0ddb2ce3242873400` |
| `src/dock157.rs` | `c433c339a96e0df63c4e964b0b48fd3d8ffc7862a1955b695822def3e9cfcaf7` | `44363f2694a1189bc1536c59cd45f13cc4939c14751aa7ba74aa2ee55f3a0af6` |
| `src/lib.rs` | `1a0ea083ad9f17985e2eb20328fb073e5ce209b1690ab14e5915e7ea613254a5` | `32b90cf67b10bcb9f7782801d834ba2c480b5c023afc8966cf8655c0f528ecb1` |
| `src/render.rs` | `2f59879a5b1b7ab2565b5b9d7631016768c0a98ceaf81e5929256643b57857cf` | `6bf15a3cda9b3a52bf897e7e3d19c93dd69c94bfd23588082366abff919856d3` |

```sh
git -c core.autocrlf=false apply --check --whitespace=nowarn /path/to/dock-cursor158-engine.patch
git -c core.autocrlf=false apply --whitespace=nowarn /path/to/dock-cursor158-engine.patch
CARGO_BUILD_JOBS=1 cargo build --offline --release --target wasm32-unknown-unknown --features ee2-pangea-huge10-dock157
```

Keep the existing private geometry beside the source root. The archived shore input stays at `../private-geometry/huge10-dock-shore.bin` with SHA-256 `33a9af433f1fe930afd84cdf0bdae749747cadf2d43ef8294afd48ef144828fc`. The resulting local/Mac private WASM SHA-256 is `f90a8cc17644879f03e16c0c0bb413ef0a43f732613307086f2de15b67d9df45`.

This transfer reproduces the prior angle used by the original automatic Dock cursor. The private browser entry keeps the explicit manual rotation override. The archived terrain bytes, private WASM and private entry files are delivered locally and in the Mac package.
