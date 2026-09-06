# Authored deformation frames and glTF direction morphs

The `gltf2-morph-frames-v1` profile imports POSITION, NORMAL and TANGENT morph
displacements, including normal-only and tangent-only targets. Deltas are additive
VEC3 float arrays applied before skin and node transforms. Their base semantic
must exist and counts must match base vertices. Tangent handedness is not a delta.
Sparse source arrays use the existing checked decoder. Missing semantics preserve
base values. Unsupported morph semantics and missing base normals fail explicitly.

Snapshot v13 adds optional normal/tangent direction offsets to each typed morph,
and an optional per-entity `shading_frames` map to animation state. A frame binding
names its topology digest, normal attribute ID and optional tangent/sign pair.
Direction offsets name an attribute ID and stable point or corner IDs according to
that attribute's domain. Runtime indices are never durable correspondence. Mesh
attribute identity, topology, domain, type, finite values and bound semantic are
validated before publication. Ambiguous duplicate shading semantics are rejected;
all authored tangent components must be bound together.

The explicit `affine-lbs-authored-frames-v1` policy adds weighted morph directions,
then transforms normals by the inverse transpose of the per-point weighted linear
skin matrix and tangents by that matrix. Its determinant adjusts handedness.
Directions are normalized before interpolation. Existing rendering orthogonalizes
the interpolated tangent against the normal and retains its deterministic fallback
for a collinear tangent. Translation does not affect directions. The source mesh
node's transform retains the existing glTF skin behavior; it does not move the
skinned output independently of the joint transforms.

Blended matrices with determinant magnitude below 1e-12, nonfinite directions and
squared direction lengths below 1e-24 return typed errors. This is an explicit
numeric profile, including rejection of singular blends of otherwise valid joints.
The authored revision remains unchanged on evaluation failures or cancellation.
Direction morph/skinning loops check cancellation; deformation is disposable output.
Native API state admits at most 256 frame bindings, within existing document,
geometry and animation budgets. Source imports retain the 131072 total offset
limit, now counting all three semantics, and the existing 64-target-per-primitive
and 20000 point/corner limits.

Entities with imported direction targets select this policy. Other deformed
entities retain the existing geometric-normal policy. Native clients can explicitly
bind frames for skin-only or position-only deformation through ordinary animation
transactions. The new optional fields are omitted when absent; older accepted
sources and snapshots retain canonical bytes and their previous evaluation policy.
Version 12 and earlier reject new fields rather than ignoring their meaning.
New evaluation digests and reports name the authored-frame policy when used.

Rust CPU, native Metal and browser WebGPU render the shared evaluated geometry.
GPU texture storage still rounds mip samples to binary16; this is separate from
deformation math. Acceptance compares strict direction passes against an independent
binary16 normal-map oracle and checks full f32 CPU/GPU radiance separately. No new
runtime dependencies or GPU kernel changes are needed for these evaluated frames.

The source contract is [glTF 2.0 sections 3.7.2.2 and 3.7.3](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets).
See the [original fixture](../fixtures/morph-frames/README.md). Morphing UV/color
attributes, general source export, dual-quaternion skinning and smooth-normal
reconstruction for missing normals remain separate profiles. M09 stays deferred.

[Acceptance evidence](../evidence/morph-frames/README.md) records complete workflows,
resource use, dependency review, numeric oracles and prior-profile identity.
