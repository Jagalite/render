# Sparse accessor implementation and acceptance plan

Primary specification read live: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
Sections 3.6.2.3/4, 5.1.1/2/3/4, 5.2/3/4 and mesh attribute table.

Bounded scope: sparse accessors across already-supported geometry, index, animation,
skin and POSITION-morph roles; core normalized unsigned byte/short TEXCOORD_0.
Additional UV sets/alpha/normal morph/export follow separately because their
renderer or schema boundaries are wider. Do not claim all normalized attributes.

Implementation candidate: Accessor uses borrowed dense bytes or an owned compact
expanded buffer (std::borrow::Cow) plus existing validated stride/count/component.
Sparse path allocates only after count*element <= explicit 16 MiB cap. Missing
bufferView implies zero base; byteOffset is forbidden without bufferView. Copy
interleaved base elements into tightly packed storage, overlay strictly increasing
unsigned u8/u16/u32 indices, require index<count. Reject sparse view byteStride or
target, invalid extensions, offsets/alignments, counts and finite-value violations.
Preserve original metadata; do not fetch resources. Scope supports vector/scalar
and f32 MAT4 roles already used, not integer matrix padding extensions.

Factor checked view reading for base/sparse data; enforce both relative and
absolute component alignment, and buffer-view bounds. Explicit normalized FLOAT
or UINT rejects. Allow normalization only per role (UV unsigned u8/u16 and existing
weights); positions/normals/tangents remain float unless later extension supported.
No schema/dependency changes. Publish a new named accessor capability separately
from unchanged historical import profiles.

Tests: equivalent sparse/dense geometry/UV/skin/morph/animation evaluated values,
interleaved base, zero base, every index width, boundary/duplicate/unsorted/out-of-
range indices, truncated values, forbidden stride/target/offset, malformed normalized
flag/components, unaligned relative-but-aligned-absolute offsets, budget bomb.
Render sparse and dense original fixture versions with independent analytic data.
Native CLI import/save/reopen/render, browser GLB import/OPFS CPU/WebGPU and shared
Wasm tests. User-authored native state remains unchanged on malformed import,
cancellation/stale/retry mismatch. Preserve all original fixture hashes/goldens.
Resource inventory includes source bytes and expansion bounds, process RSS/footprint.
