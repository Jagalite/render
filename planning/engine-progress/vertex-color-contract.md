# Completed bounded input increment: linear glTF COLOR_0

Implemented after source-conformance correction 99854ea. Final semantics and
acceptance are in docs/vertex_colors.md and evidence/vertex-colors/README.md.
Primary specification read 2026-09-06: COLOR_0 is a linear base-color multiplier;
VEC3 implies alpha1; float and normalized u8/u16 supported; all components must be
clamped to [0,1] before use. Emission is not multiplied. OPAQUE ignores alpha;
MASK/BLEND coverage multiplies base factor, base texture alpha and vertex alpha.

Implemented typed design: add AttributeValues::Vec4 and semantic color_rgba, point
or corner domain, one such attribute per mesh, imported COLOR_0 -> stable Id200.
Snapshot v14 gates Vec4 schema. Unlike per-material selectors, an optional geometry
color naturally supports one source material shared by colored and uncolored
primitives. Restrict rendering color_rgba to PBR until legacy diffuse consumers
are deliberately extended; return typed unsupported_profile rather than ignoring
it. Missing color defaults to white. General custom Vec4 attributes remain typed
and require explicit transfer policies. Preserve old snapshots/serialized bytes.

Geometry holds optional color attribute ID and Triangle optional [Vec4;3]. CPU
shading and alpha interpolate with existing barycentrics. GPU appends three vec4
rows after UV rows, sets first triangle position's unused w to color offset (0
means white), and includes these rows in existing variable stride and exact index
budgets. The generated Rust kernel multiplies PBR base RGB by bary_color.xyz;
no instance color switch is needed. Emission and normals remain independent.
GPU alpha remains explicitly unsupported until later transport work.

Audit all AttributeValues matches in core modeling/geometry/interchange/UV tools;
no wildcard drop. Bake base-color samples multiply interpolated vertex RGB; emission
bakes do not. Preserve RGBA during displacement by barycentric corner transfer
and stable IDs, or explicitly reject that combination if bounded transfer cannot
be completed in this increment. Color seams require corner-domain preservation.
Morph/LBS keeps base colors unchanged; COLOR morph targets stay unsupported.

Positive original fixture: smooth RGB barycentric gradient, constant texture/factor,
shared source material with one uncolored primitive, RGB alpha default1, RGBA
coverage, normalized u8/u16 and sparse equivalents. Include out-of-range float
clamping with conversion-report count, negative/malformed/count/domain cases,
archive/OPFS, stale/cancel/budget checks, CPU/native/browserGPU analytic and pixel
comparisons, default-profile exactness and official Khronos source validation.
No runtime dependencies. Fixture corrections must be reviewed as source semantics,
never regenerate renderer golden images.
