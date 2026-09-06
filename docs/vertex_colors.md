# Linear vertex colors in the bounded PBR profile

`gltf2-vertex-color-v1` admits glTF COLOR_0 as a linear base-color multiplier.
VEC3 supplies alpha=1; VEC4 preserves alpha. FLOAT and normalized unsigned byte or
short components use the bounded dense/strided/sparse accessor decoder. Finite
source components are clamped to [0,1] before interpolation, with a component count
in the conversion report. Nonfinite values, unsupported component types, bad
counts and additional color sets fail explicitly. Indexed source names retain the
canonical consecutive-set contract.

The semantics follow the [Khronos glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html):
linear vertex RGB multiplies the base factor and the decoded base texture. It does
not tint emission, metallic/roughness, occlusion or normal maps. An absent color
attribute supplies white, including on a primitive sharing a colored primitive's
material. The source material is not duplicated just to apply geometry colors.
CPU MASK/BLEND coverage multiplies vertex alpha, factor alpha and texture alpha.
OPAQUE ignores their alpha components. The existing stochastic BLEND and base-mip
CPU alpha approximation remains explicit. GPU alpha is a separate pending gate.

Snapshot v14 adds the typed `vec4` attribute value variant; older snapshot versions
reject it. Native `color_rgba` attributes use stable IDs, point or corner domains,
exact domain counts, finite [0,1] values, and at most one color attribute per mesh.
Imported COLOR_0 has stable attribute ID 200 and linear transfer policy. Native
out-of-range values fail transaction validation rather than being silently changed.
Other Vec4 semantics remain typed general attributes with declared transfer policy.
Ordinary PutMesh transactions and existing archive/OPFS storage persist colors.
No privileged mutation path or public material field is added.

Evaluation copies color values into immutable triangles. CPU hits, alpha and
albedo baking interpolate barycentrically and clamp the result. Emission baking
remains independent. Modeling transfers all four components under the declared
policy; invalid transferred output is rejected by normal mesh validation.
Displacement preserves stable color IDs and corner seams through bounded linear
RGBA transfer. Colored displacement uses the explicit `uniform-rgba-uv-v1` policy;
colorless displacement retains the previous policy and receipt bytes. Morphing and
skinning leave base colors attached to their existing topology; color morphs remain
unsupported. Legacy diffuse rendering and the flat raster preview reject colored
geometry with a typed unsupported-profile error. Existing geometry-only OBJ/glTF
export reports the omitted RGBA attribute explicitly; native archives retain it.

The GPU adds three f32 vec4 rows only to colored triangles, after their UV rows.
A private offset and existing variable leaf stride describe these rows. The
Rust-authored typed kernel generates barycentric color lookup; no maintained shader
language source is introduced. Packed byte/index limits include the new rows.
Existing RGBA16 texture precision remains distinct from f32 vertex color storage.
Missing colors retain old packed bytes. Public schemas expose stable IDs, never
these private row indices.

The original [fixture contract](../fixtures/vertex-colors/README.md) separates
analytic color/coverage oracles, valid source fixtures and malformed in-memory test
variants. The independent Khronos validator is a development oracle only. Runtime
Cargo dependencies and features are unchanged. This remains an experimental bounded
profile, not a claim of arbitrary glTF or editor support.

[Acceptance evidence](../evidence/vertex-colors/README.md) records 134 native and
115 Wasm tests, ten Metal comparisons and five complete native/browser workflows.
