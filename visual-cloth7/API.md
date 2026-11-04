# Own visual cloth module

This dependency-free Rust crate implements the bounded, explicitly named
PC53/RC_CHOP arithmetic model in `DOMAIN.md`. It has no game, network, file,
original executable or operating-system imports. It does not establish the
original game's complete temporal FPU configuration.

Build from the project root with `python tools/build_source_cloth.py`; this
also updates the browser module's size and SHA-256 pin. The complete build
scripts do this automatically. Run this crate's native checks with
`cargo test --offline -- --test-threads=1` from this directory.

`cloth_abi_revision()` is1. All numeric slot IDs are private visual slots,
never simulation entity handles. The browser adapter in
`docs/source-cloth-wasm.js` keeps those slots private. Its controller retains
the real world session, full generational actor handle and source visual.

- `input_alloc(points*3)` and `pins_alloc(points)` allocate scratch space.
  Reacquire memory views after either call. Copy original-local binary32 XYZ
  and0/1 pins, then call `cloth_create(nx,ny)`. Invalid creation returnsFFFFFFFF.
- `cloth_step(slot,deltaF64,gx,gy,gz,minX,minY,minZ,maxX,maxY,maxZ)` uses explicit
  gravity and wind endpoints. Inputs stayf64 across the ABI until their
  source-defined stores. A premature JS f32 conversion would change the model.
- `cloth_forces` configures those vectors for a slot. Optional
  `cloth_step_all(deltaF64)` rejects before any update if any live slot lacks
  forces. The browser controller currently uses individual steps.
- `cloth_light(slot,uniform,ambientRGB,diffuseRGB,sourceRayXYZ)` uses undimmed
  original source RGB and the original source-axis ray. Normal mode is0;
  uniform mode is1. Local normals are diagnostic outputs.
- `cloth_positions`, `cloth_velocity`, `cloth_mass`, `cloth_inverse`,
  `cloth_colors`, `cloth_normals`, `cloth_pins` and `cloth_len` expose bounded
  slot buffers. The caller treats them as read-only and reacquires views after
  growth, reset or disposal. The JS adapter copies completed frame outputs.
- `cloth_release(slot)` clears pins and leaves physics running.
  `cloth_drop(slot)` disposes one slot; `cloth_reset()` disposes all slots.
  These functions do not infer a native Death event.
- `cloth_seed`, `cloth_steps`, `cloth_word`, `cloth_delta`, `cloth_half` and
  `cloth_wind(slot,axis)` provide own diagnostics for independent oracles.

The browser runtime owns pause, frame, visibility and load/reset scheduling.
Its missing-module or integrity-failure path keeps verified static original
sails available. The source Flag material and native-color conversion limits
are documented in `docs/SOURCE-SAILS7.md`. Simulation saves and multiplayer
hashes never include this independent visual state.

Correctness evidence includes exact Fraction/integer boundary oracles and
the frozen source-kernel/lighting cases. Performance measurements depend on
the host and exclude uploads and the rest of the game;50 mixed ships took
about18.5ms for physics alone in one paired run. This is no frame-rate guarantee.
