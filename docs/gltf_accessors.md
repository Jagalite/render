# Bounded sparse glTF accessors and normalized UV0

The `gltf2-sparse-uv0-v1` capability extends the existing static/animated import
profiles. It accepts sparse overrides for the already-supported accessor roles:
triangle positions/indices, normals/tangents, skin joint indices/weights, inverse
bind matrices, clip channels, and POSITION morphs. TEXCOORD_0 additionally accepts
normalized unsigned byte or short values. More UV sets, vertex colors, quantized
position/normal extensions and normal/tangent morphs retain separate gates.

This is an input adapter extension. The existing typed native meshes, attributes,
textures, clips and rigs retain their meaning; no snapshot, request, report or ABI
layout change is required. Converted points and UVs use existing f32 native storage.
Dense and sparse encodings with equal values therefore produce the same geometry
content identity. Source-scoped object/clip IDs still differ when source bytes
change. Authoring, persistence and rendering follow ordinary transactions and
exact-time APIs; no imported metadata executes or fetches resources.

The decoder keeps borrowed dense data where possible. Sparse data is checked then
expanded into a bounded compact buffer. Base values can be interleaved or implicit
zeros. All sparse indices are checked before copying replacements; they must be
ordered, distinct and within the accessor count. Byte/short/int sparse indices are
supported. Offsets, range arithmetic, shape, component widths and finite floats
are validated. Sparse views reject stride/target declarations and extensions.
Both relative and absolute component alignment are enforced, along with four-byte
base vertex offset/stride alignment. A missing base view forbids byteOffset.

Each accessor admits at most 196,608 elements and at most 16 MiB of expanded bytes.
Current supported MAT4 float data reaches 12 MiB at that count limit. Sparse index
validation stores at most one usize per replaced element. Compact zero storage is
reused for overlays. These temporary decoder limits supplement existing 4 MiB
source bundles, point/corner, morph, skin, key, snapshot and transaction budgets;
they are not a whole-process allocator limit. Synchronous bounded import parsing
retains the existing cancellation boundary before publication.

Normalization is role-specific. UV unsigned values divide by 255 or 65,535; the
existing normalized skin-weight path retains f64 weight normalization. FLOAT and
UNSIGNED_INT cannot have normalized=true. Integer positions/normals/tangents remain
unsupported without an explicit future quantization profile. No float conversion
is inferred merely because a material references an accessor.

The primary [glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
and [sparse index schema](https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/schema/accessor.sparse.indices.schema.json)
and [sparse value schema](https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/schema/accessor.sparse.values.schema.json)
define the source semantics. No Khronos computational runtime is introduced.

## Acceptance

The [original sparse fixture contract](../fixtures/sparse-gltf/README.md) supplies
three equivalent textured character encodings and both GLB/explicit bundles.
Independent closed-form deformation expectations supplement backend agreement.
Tests require exact dense/sparse geometry, decoded images and CPU pixels; actual
Metal/WebGPU comparisons use the declared precision tolerance and exact IDs within
each source. Tests also exercise missing-base zeros, every sparse index width,
malformed ranges/order/alignment/normalization, budget rejection, stable source
identities, retries, cancellation, unchanged authored state and save/reload.

`python3 scripts/validate-sparse-gltf.py` records fresh commands/logs, source hashes,
resources and dependency inventory. It runs full native/Wasm tests, portability
compilation, release browser bindings, the existing animated/shutter workflows,
and independent CLI/browser clients. Runtime capability status is updated only
after those workflows pass. No existing image golden is regenerated.
