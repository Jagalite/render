# Tiled painting engine contract

Implemented on qualified UV baseline be20d1f. Existing material texture bindings select
immutable encoded images with explicit sRGB-color/linear-data roles and stable UV
attribute IDs. Do not overload those IDs with mutable paint canvases or replace
encoded asset semantics. Painting owns a typed editable source; an explicit bounded
bake/composite publishes a normal ImageAsset and material binding transaction.

First interface: immutable paint surface assets with metric-independent pixel
extent, fixed tile size, color role, stable layer IDs, blend mode, opacity and
sparse tile hashes. Typed tiles contain finite premultiplied linear RGBA, not an
untyped map. Transparent missing tiles have explicit meaning. Layer/mask identity
and canvas dimensions are validated. Keep public tile coordinates independent of
private tile-buffer layouts. The initial maximum image extent should respect the
existing 2048-pixel output image limit; bigger virtual textures require a separate
renderer paging contract. Texture asset updates must preserve shared materials
unless the caller explicitly chooses a shared-material mutation.

Brush requests pin document revision and source paint-asset hash, target layer,
explicit pixel-space positions/radii, sampled pressure/time and color role. Round
brush coverage and spacing semantics need an analytic definition, explicit clipping
and deterministic accumulation. Store a typed stroke recipe and resulting changed
tile identities. Bound samples, tile touches, pixel work, delta bytes and retained
assets. Recompute only touched tiles, reuse all other immutable tile references.
Native/core cancellation checks before allocation, per tile/row and before ordinary
transaction publication. A synchronous browser entrypoint is bounded baseline;
do not claim browser-event cancellation during a synchronous operation.

Masks/layers, sparse changes and recovery are essential to this gate. Brush preview
and gesture chunking remain editor concerns; expose checkpoints through ordinary
commands so tests can recover and undo a long stroke from retained asset identities.
Support an explicit erase mode and deterministic mask multiplication. Projected
painting is separate: it must pin geometry/camera/evaluation revisions and use the
existing spatial queries; do not invent approximate screen-to-UV behavior.

Bake/composite traverses the canvas into a bounded normal image asset with explicit
color encoding/quantization report, then attaches it via the normal material
transaction. Native and browser must match stored tiles, image bytes, revisions and
CPU output for the original fixture. Add an end-to-end UV unwrap/pack -> layered
paint -> image bake -> material binding -> CPU/Metal/WebGPU render -> save/reopen
workflow, plus tile-locality, color/alpha, layer order, mask, permission, stale,
idempotency, quota/publication failure and cancellation tests.

No new computational dependency is initially needed: current Rust image and PNG
encoding and explicit color conversion can supply the bake adapter. Review exact
alpha and transfer-function math from existing imaging code before deciding tile
representation. Measure actual touched-tile bytes and allocation/process behavior;
do not claim sparse speed from source inspection. The bounded profile is qualified in `evidence/tiled-painting`.

## Representation and numerical decisions for implementation

Profile `paint-tiles-v1`: 32 by 32 immutable tiles. Color is premultiplied linear
RGBA16_UNORM; masks are scalar UNORM16. JSON payloads are lowercase hexadecimal
little-endian u16 bytes in row-major pixel order, RGBA channel order. This encoding
is independent of Rust memory layout. Tile hashes omit placement coordinates.
Layer tile references are a sorted sequence of typed {coordinate, tile} records;
duplicate coordinates are rejected. Missing color is transparent black; missing
mask is full coverage. Out-of-canvas edge pixels must equal these defaults.
Reject non-premultiplied colors, wrong lengths, unknown versions/fields and hashes.

Canvas extent is 1..2048 per axis. At most eight uniquely identified layers in
bottom-to-top order; each has a stable ID, name, visibility, UNORM16 opacity, color
tiles and mask tiles. At most 256 references of each tile kind per canvas and 512
retained tiles per snapshot, with 64 retained canvases. These are independent caps:
large dense multilayer canvases can exceed the profile. Layer masks multiply layer
coverage at composite time, not stroke coverage. Erasing changes color/alpha;
painting a mask changes scalar coverage without destroying the layer's color.

Use mul16(a,b) = floor((a*b+32767)/65535). Source-over is source plus destination
multiplied by one minus source alpha, for all four premultiplied channels. Layer
opacity and mask apply before composition. Mask editing interpolates old coverage
toward a target scalar. No HDR, alternative blend modes, tilt, projection or UDIM
fields are accepted by this profile.

A round brush has radius 0.25..128 pixels, hardness 0..1, and spacing 0.05..2 times
radius. Four fixed subpixel centers (0.25 or 0.75 on each axis) measure coverage:
full inside hardness*radius, linear falloff to zero at radius. Average coverage is
multiplied by interpolated pressure and quantized to UNORM16; paint or erase opacity
then uses mul16. Clip work to the canvas. Count even out-of-canvas dabs toward limits.

Samples carry pixel positions (absolute coordinate <=4096), pressure 0..1 and
nonnegative monotone rational time. Resample line segments at fixed arc spacing;
interpolate pressure. Spacing positions within 32*f64::EPSILON*max(1,segment_length)
pixels of a segment endpoint snap to its exact authored position and pressure; this
prevents a roundoff-induced second endpoint dab. The first sample emits a dab.
Zero-length segments update input tail without a dwell dab. Retain at most sixteen
typed stroke recipes. Re-derive the bounded dab prefix from all retained samples at
each checkpoint, applying only newly emitted dabs; this avoids trusting a serialized
floating-point accumulator. A checkpoint does not force an endpoint dab; explicit finish
emits an endpoint only if it has not already been emitted. Continuing an open stroke
requires its stable ID, same brush and layer. Reject closed-stroke continuation.
Limit each retained stroke to 256 samples, each request to 4096 dabs, 16,777,216 pixel
coverage evaluations and 64 touched tiles. Output bytes are also explicitly bounded.

Bake is a separate dense operation. Composite in linear premultiplied UNORM16,
unpremultiply explicitly and encode a normal straight-alpha RGBA8 PNG. Transparent
RGB is zero. Linear-data role uses direct quantization; sRGB-color uses a deterministic
transfer function in the existing pure Rust libm dependency, with error measured in
premultiplied space. Native/Wasm byte equality must be demonstrated before acceptance.
General floating-point transfer sources and existing image goldens stay unchanged.
Decoded sRGB8 and mip rounding use the explicit portability profiles in ADR-036/037.

## Fixture contract

Construct the scene from existing native primitives and UV operations, then author
opaque/transparent layers, paint across tile boundaries, mask, erase, checkpoint,
recover, bake and explicitly bind the image to a named UV set. Retain original
requests and reports. Independently check analytic blend/alpha and mask values,
layer ordering, untouched tile identities, one-shot/checkpoint equivalence and
quantization boundaries. Reject malformed payloads, overflow/work limits, stale
sources/revisions, permissions, closed strokes and publication failures. Check pre,
mid and late cancellation. Compare native/Wasm tiles, image bytes and revisions,
CPU pixels and tolerance-bounded GPU outputs, plus native archive and OPFS reopen.
Existing fixture inputs, shaders, render passes and receipts must remain unchanged.
This bounded workflow passed; see `evidence/tiled-painting/README.md`.
