# Named texture-coordinate bindings

The bounded `gltf2-named-uv-v1` profile admits TEXCOORD_0 through TEXCOORD_7.
Each texture role selects its own coordinates. Float and normalized unsigned-byte
or unsigned-short arrays use the existing checked accessor decoder, including
sparse expansion. Referenced sets must exist on every assigned primitive; missing
sets and unsupported indices fail before publication.

A texture binding optionally names a mesh-local stable `uv_attribute` ID. Omission
retains the existing first-UV behavior and serialized bytes. glTF UV0 keeps ID 2;
UV1 through UV7 use IDs 101 through 107. These IDs identify typed corner Vec2
attributes, independently of private render slots. Explicit selectors require
snapshot version 12. Earlier snapshots without selectors retain their version.
The Rust command enum boxes its material payload to keep its variant size bounded;
Serde JSON and command identity do not change from that indirection.

The CPU evaluator and generated Rust GPU kernel interpolate the selected set with
hit barycentrics. Primary texture footprints use that set's projected coordinates;
secondary texture lookups retain base-mip sampling. Missing authored tangents use
the normal texture's chosen UV basis, including its handedness. Authored tangents
keep their source meaning. Alpha coverage uses the base-color binding selection
and its existing base-mip CPU approximation. GPU alpha remains unsupported.

Evaluated geometry retains at most eight corner UV sets once per geometry, shared
across instances. GPU triangles append two vec4 rows per retained set; leaf metadata
carries triangle stride, and each material descriptor carries a private selected
slot. Existing exact-f32 address and device buffer budgets still apply. Rendering
rejects missing or malformed evaluated slot tables before sampling.

The `uniform-named-uv-v1` displacement policy selects the image-height binding's UV
set and transfers every UV set by barycentric interpolation, preserving names and
stable IDs. Wave displacement continues to use default UVs. Normals are recomputed
geometrically as before. This policy intentionally changes displacement-derived
attribute identities and receipt digests, including single-UV meshes; source
snapshots remain unchanged. Subdivision, vertex amplification and cancellation
limits are retained. The cache key includes the source geometry and displacement
policy inputs; preservation is independent of other material bindings.

Existing UV baking still targets the default atlas. Its material samples use each
explicitly selected source set, with pixel differentials transformed through the
atlas-to-selected-coordinate Jacobian. Selecting a different bake destination is
a separate operation profile. Geometry-only interchange exporters remain separate
lossy profiles; each omitted additional UV set is identified by name and stable ID
in the conversion report. Document archives preserve all named bindings and attributes.

The source contract is the official [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html).
The [original fixture](../fixtures/named-uv/README.md) contains independent UV-role
and normal-frame oracles. No runtime dependencies are added. Vertex colors,
texture-transform extensions and general material/animated source export remain
separate gates. M09 stays deferred.

[Acceptance evidence](../evidence/named-uv/README.md) covers the complete native
and browser workflow, analytical cases, resource use and dependency review.
