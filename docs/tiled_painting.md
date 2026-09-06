# Tiled painting: paint-tiles-v1

This bounded M10 engine profile passed its [acceptance gates](../evidence/tiled-painting/README.md).
Qualified UV authoring remains available independently. Painting does not complete M10's sculpting, remeshing,
retopology or M09 interactive demonstration requirements.

The engine edits an independent canvas, then explicitly bakes it to a normal image.
A caller selects that image and a named UV attribute through an ordinary material
transaction. Editing or baking a canvas never silently changes a shared material.
All maintained computation is Rust, shared by native and browser. No remote service,
new Cargo package, shader change or computational FFI is introduced.

## Workflow and public operations

Create a canvas with `put_paint_asset` and `set_paint_canvas` in an ordinary `apply`
transaction. Its stable canvas ID binds an immutable asset hash. Layers have stable
IDs and explicit bottom-to-top order. A `put_paint_asset` plus compare-and-replace
`set_paint_canvas` also changes layer names, opacity, visibility, order or retained
recipes. Referenced layers must exist; removing one requires removing its recipes.

`author_paint` pins a document revision, canvas ID, source asset hash, target layer,
stroke ID and typed brush samples. It prepares changed tile payloads and a new asset,
then publishes ordinary `put_paint_tile`, `put_paint_asset` and `set_paint_canvas`
commands. Every entrypoint retains permissions, revision checks, idempotency and
transaction budgets. Replaying an accepted request returns its original receipt,
including after later edits, while the pinned source remains retained.

`bake_paint` pins the same source identity and publishes an ordinary `ImageAsset`.
Its report includes source and image hashes, role, pixel work, encoded size and
maximum premultiplied quantization error. A later explicit `put_material` chooses
that image, sampler and stable UV attribute ID. Copy a shared material to a new ID
and use `set_material` when only one instance should change. The generic document
operations remain available in the CLI and browser API. Baked alpha follows the
chosen material's existing opacity mode; binding an RGBA image does not implicitly
turn an opaque material into a transparent one.

To undo or redo a checkpoint, compare-and-replace the canvas binding with a retained
prior asset hash. Saved source assets and tile identities remain immutable. Native
publication uses the existing journal/store transaction; browser save/reopen uses
OPFS through the existing document API. Failed publication must leave both document
and saved records unchanged. Snapshot 18 contains optional omitted-when-empty
`paint_tiles`, `paint_assets` and `paint_canvases` tables. Earlier clients must reject
painting archives/checkpoints. Documents without painting keep their prior bytes.

See [the strict wire schema](../schemas/tiled_painting.schema.json),
[agent request schema](../schemas/agent_request.schema.json),
[fixture contract](../fixtures/tiled-painting/README.md) and
[interface review](../planning/engine-progress/tiled-painting-interface-review.md).

## Color, tiles, layers and masks

A tile is 32 by 32 pixels. Color tiles store linear premultiplied RGBA16_UNORM;
mask tiles store scalar UNORM16 coverage. Public JSON uses lowercase hexadecimal
little-endian u16 bytes, row-major pixels and RGBA channel order. Hashes omit tile
placement. Typed layer references supply integer tile coordinates and hashes;
duplicate coordinates reject. Missing color is transparent black, while missing
mask is full coverage. Partial boundary tiles must contain those defaults outside
the canvas. Colors with RGB greater than alpha reject.

The canvas role is explicit: `srgb_color` or `linear_data`. Brush colors are always
straight linear input in 0..1, including on an sRGB-role canvas. HDR and alternative
blend modes are outside this profile. Integer multiplication is
`floor((a*b + 32767)/65535)`. Source-over uses premultiplied source plus destination
times one minus source alpha. Layer opacity and mask multiply layer coverage before
composition. Masks are non-destructive layer masks; they do not stencil brush input.
Painting a mask interpolates its scalar coverage toward the requested value.
Erasing multiplies all four stored color channels by remaining coverage.

Baking composites in linear premultiplied UNORM16, explicitly unpremultiplies and
quantizes to straight RGBA8 PNG. RGB is zero if encoded alpha is zero. Linear-data
RGB quantizes directly; sRGB-color RGB uses the explicit sRGB transfer function.
The already pinned pure Rust libm supplies bake transfer powers and brush distances.
The error report measures reconstructed linear premultiplied channels, including
alpha; straight RGB is undefined at zero alpha. The filtering algorithm and general
floating-point transfer functions remain unchanged. Mip selection uses the shared Rust f64 `log2`, rounded to f32, which
matches all 1,041 captured native footprints and removes platform `log2f` rounding
differences. Decoded 8-bit sRGB images use the portable `srgb8-compat-v1` table, which
preserves all 256 qualified native values and removes platform power-function
rounding differences. Its error against a high-precision analytic transfer is
bounded by 3e-7; generating inputs and the native numeric oracle are retained.
Bake output is dense, even if edits are sparse; there is no sparse renderer
texture paging claim.

## Brush input and checkpoints

Pixel coordinates address the canvas directly. Radius is 0.25..128 pixels, hardness
is 0..1 and spacing is 0.05..2 times radius. Four fixed subpixel samples at quarter
or three-quarter pixel offsets measure coverage. Coverage is full inside
hardness*radius, then falls linearly to zero at radius. Average geometric coverage
times interpolated pressure is rounded to UNORM16; paint/erase opacity uses the
integer multiplication rule. Work is clipped to the canvas.

Samples carry bounded pixel position, pressure and canonical nonnegative rational
time. Time must be monotone. The initial sample emits a dab. Line segments resample
at fixed arc spacing, interpolating pressure. Zero-length segments update the input
tail without a dwell dab. A checkpoint keeps the stroke open and does not force an
endpoint dab. Explicit finish emits the endpoint only if it has not been emitted.
Spacing positions within `32*f64::EPSILON*max(1,segment_length)` pixels of an endpoint
snap to its exact authored position and pressure, preventing a rounding-induced
second dab. Continuing requires the same open stroke ID, brush and layer.

The asset retains bounded typed stroke recipes. Each continuation re-derives its dab
prefix from all authored samples and applies only newly emitted dabs. It does not
trust serialized floating-point accumulator state. Interleaved operations execute
in transaction order. One-shot versus checkpoint equality assumes the same ordered
samples and no intervening change to the affected pixels. Recipes record accepted
input; the immutable tile results are the authoritative edited content.

## Resource and qualification boundary

| Resource | Profile maximum |
| --- | ---: |
| Canvas width or height | 2048 pixels |
| Layers per canvas | 8 |
| Color references per canvas | 256 |
| Mask references per canvas | 256 |
| Retained tiles per snapshot | 512 |
| Retained assets / canvas bindings | 64 / 64 |
| Retained stroke recipes per asset | 16 |
| Samples per recipe | 256 |
| Generated dabs per stroke computation | 4096 |
| Touched tiles per stroke request | 64 |
| Stroke pixel coverage evaluations | 16,777,216 |
| Stroke delta bytes | 4 MiB |
| Bake pixel-layer work | 16,777,216 |
| Encoded bake output | 4 MiB |

These caps are independent. A dense 2048-pixel canvas or many dense layers can exceed
reference, work, retention, storage or agent snapshot limits. Color tile payloads
are 8192 decoded bytes and mask tiles 2048; hexadecimal wire payloads double those
bytes before metadata. Reports distinguish touched tiles, changed coordinates,
copied tile bytes, output bytes and pixel work. No speed claim follows from these
counts. Persistent journals and dense bakes have separate measured amplification. The
current journal adapter stores complete document envelopes; sparse tile computation
does not imply sparse physical journal writes.

Cancellation is checked before preparation, during resampling/rows/tiles and before
ordinary publication. PNG encoding and normal document validation are bounded
synchronous work. The browser baseline is synchronous admission; a browser event
cannot interrupt an already running synchronous stroke. GPU previews retain their
existing asynchronous cancellation path.

Projected painting needs a separate geometry/camera/evaluation revision contract.
Tilt, UDIM, virtual textures, alternative blend modes, HDR, arbitrary input images
as editable layer sources and general multichannel/material baking are extensions.
Unknown or unsupported fields reject. Qualification covers the measured bounded
workflow, with native/Wasm, fixture, storage, renderer and provenance evidence.
It does not extend support to these separate capabilities.
