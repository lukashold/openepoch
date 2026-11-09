# Optional private Huge10 Dock territory update

Apply this patch over the exact DockCursor158 private engine. The previous optional patch sequence starts with the private Huge10 Resume131 engine and applies Dock157 followed by DockCursor158. The standard and Valley engines retain their own build settings.

| Source path | DockCursor158 SHA-256 | DockTerritory161 SHA-256 |
| --- | --- | --- |
| `src/dock157.rs` | `44363f2694a1189bc1536c59cd45f13cc4939c14751aa7ba74aa2ee55f3a0af6` | `cf7aa84421d6c221e9764b34f3d507ea340950c089625a048b53dcd0f2c3ed0e` |
| `src/placement.rs` | `4046a8b5f7f614f076ac5449395180b7117850e86968b6893e292fa74e7db252` | `d898ef31c41f158c26d37ca7094cfb7eda2d4203f9443405add10410997fce99` |

Run from the private engine source root.

```sh
git -c core.autocrlf=false apply --check --whitespace=nowarn /path/to/dock-territory161-engine.patch
git -c core.autocrlf=false apply --whitespace=nowarn /path/to/dock-territory161-engine.patch
CARGO_BUILD_JOBS=1 cargo build --offline --release --target wasm32-unknown-unknown --features ee2-pangea-huge10-dock157
```

Keep the inherited private geometry beside the source root. The added build input is `../private-geometry/huge10-dock-territory.bin`. It contains 62,500 row-major bytes. Native unassigned cells use zero. The SHA-256 is `4b43fc5a2403e4f87b01f93525f09e23ada9a66343076db0cbcea3b86d008599`. The optional Rust test requires the detached x86 fixture `../private-geometry/dock-territory161-original.txt` with SHA-256 `7c5bc5f4da92f72493961bcd092084b6f9d95ac5d9d5067b237e90cb5f77a6dd`.

The resulting private WASM SHA-256 is `22237e854c46283ff31b8bc147441df998ac779218e30d3c92c292eb0e127dd4`. The archived map matrix, private compiled carrier and original lighting media stay in the local/Mac delivery.

The automatic Dock uses the original radius-four and radius-five region lookup for authenticated opening territories. The scope is the default private Huge10 map. Manual placement and ambiguous region mappings retain the inherited behavior.
