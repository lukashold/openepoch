# Arithmetic domain and exact fallbacks

This is an independent visual kernel with a **named PC53/RC_CHOP model**.
It does not prove the original game's control-word lifetime or reproduce the
larger x87 exponent range outside the bounded cloth calculation. No original
engine routine, EXE or DLL is executed.

## Generic exported arithmetic

The six `math_*` exports accept finite binary64 operands, with nonzero divisors
and nonnegative square-root inputs. They use no epsilon or error tolerance.

- FastTwoSum orders the operands by magnitude. For finite addition results,
  the two subtraction operations recover the exact residual; gradual IEEE
  underflow cannot lose a smaller nonzero addend residual because all binary64
  inputs are integral multiples of2^-1074. A finite-result addition therefore
  needs at most a one-ULP step toward zero. Overflow uses exact integer fallback.
- A product of two finite binary32 operands has at most48 significant bits.
  Its nonzero magnitude is at least2^-298 and below2^256, hence is an exact,
  normal binary64 value. Zero products retain the IEEE sign.
- Other product/division/sqrt residual fast paths conservatively require each
  input to be zero or in2^-400..2^400. Splitter2^27+1 cannot overflow there.
  The smallest two-product error component is at least2^-906, still normal.
  For division, quotient exponents may reach±800, but the product with the
  divisor remains near the original dividend; its residual components stay
  normal and its split cannot overflow. The residual sign determines which
  adjacent binary64 result is the required toward-zero result.
- The WASM `f64.sqrt` instruction is round-to-nearest IEEE square root. The
  exact residual of root² determines whether its adjacent lower value is
  needed. No assumption about an arbitrary host approximation is required.
- `FSTP32` first rounds to binary32, then decrements its magnitude encoding
  only when that rounded magnitude exceeds the binary64 input. This also
  retains signed zero and handles binary32 overflow by selecting max finite.

Outside the conservative residual domain, `math.rs` decomposes the operands
into signed integer significands and powers of two. Products need at most106
significand bits. Division and square root are quantized by exact integer
division/restoring integer square root. All useu128; no JS BigInt, floating
residual or guessed tolerance is involved in the fallback. Addition aligns up
to75 exponent bits in au128. At larger gaps the smaller nonzero operand is
strictly below one ULP: same-sign addition keeps the larger magnitude, while
opposite-sign addition selects its predecessor toward zero. Rational packing
handles normal/subnormal/zero/max-finite outputs directly. Unsupported NaN,
infinite operands, zero division and negative roots are outside this contract.

The new independent Fraction/integer oracle covers18,924 boundary operations,
including both zero signs, smallest subnormals, binade transitions, endpoints,
underflow and overflow. The frozen2,448 arithmetic cases remain exact too.

## Internal cloth specialization

`cloth.rs` calls a bounded version of the same residual formulas, avoiding
repeated general input guards. This is not a guessed fast approximation.

The constructor validates dimensions2..255, at most65,025 points, finite
binary32 positions with magnitude below1e10, and0/1 pins. Each stored position,
velocity, force, mass, inverse mass and color is binary32. Toward-zero stores
retain finite max values on binary32 overflow. Callers must treat exposed
position/velocity/mass buffers as read-only; corrupting them with NaN/Infinity
violates this private visual-state contract.

Even if a later finite state reaches binary32 endpoints, vector differences
stored before length are finite binary32; length squared is below2^260 and
nonzero length is at least2^-149. Inverse lengths are below2^150. The fixed
mass formula stays positive and bounded for the admitted dimensions. Spring
distance-minus-rest has no nonzero magnitude below2^-201; multiplying by300
and normalized binary32 components stays within the residual bounds.

Velocity drag uses speed at most2^130 and speed² below2^260. Its intermediate
nonzero scalar can be as small as approximately2^-299, but the operands given
to multiplication stay above2^-400. Products immediately stored as binary32
underflow to zero normally. Time is first stored as binary32, then clamped to
the exact source.05000000074505806 and halved. Supplied finite force vectors
have magnitude below1e10 and are stored at the same source points.

For local lighting, neighbor differences are binary32 or bounded extended
differences of binary32 points. Cross products are stored as binary32 before
normalization. Dot products use binary32 unit normals and binary32 source
ray components; a nonzero cancellation result remains above2^-350. Raw source
light components are finite binary32. Thus all multiplication/division/sqrt
operands in the internal kernel stay in the stated2^-400..2^400 domain.
Addition remains valid over its broader finite-result domain.

Debug builds assert the bounded multiplication/division/sqrt preconditions.
Two debug test groups exercise grids through255² points and extreme finite
binary32 states, smallest subnormals, pin release/forces and lighting. All
69 selected source state snapshots and31,981 lighting checks also match their
independent rational oracle. These tests supplement the domain derivation;
they are not presented as exhaustive enumeration of all floating-point states.

## JS experiment

The sibling `chop53.mjs` and copied JS kernel are **benchmark prototypes only**.
They keep the frozen bounded-domain residual fallback, give little speedup,
and are not the proposed production backend. Do not promote them in place of
the frozen arithmetic. The recommended candidate is the separate own WASM
with the documented integer fallback and explicit buffer/lifecycle adapter.
