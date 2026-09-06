# Animated glTF/GLB input profile

`gltf2-animated-pbr-v0` extends the existing opt-in PBR importer with native
clips, skins and position morph targets. It uses the same glTF/GLB entry points
and `project import` requests as static assets. Implementation is Rust shared by
native and Wasm. No conversion service or external computational dependency is added.
The [CLI contract](project_cli.md) covers paths, transactions and artifact publication.

## Supported data and conversion

- glTF 2.0 JSON with every buffer/image explicitly supplied, or GLB with its
  embedded BIN chunk. Existing opaque PBR, PNG/JPEG, geometry and camera limits apply.
- Selected-scene hierarchy, including non-joint ancestors. Source indices map to
  persistent document-scoped IDs derived from source content; they are not runtime handles.
- Translation, rotation and scale channels; STEP, LINEAR and CUBICSPLINE.
  Missing paths receive constant channels containing the source node's default TRS.
  Channels replace local TRS, including nonuniform/negative scales. A matrix-authored
  node cannot have transform animation. Quaternion linear interpolation uses slerp;
  cubic interpolation uses normalized component Hermite with derivatives per second.
- Key times are converted from float32 to exact rational seconds. Supported times
  are nonnegative, at most 1e9 seconds, with reduced denominator at most 2^63.
  Duplicate/decreasing times fail. A single-time clip receives a one-second constant
  interval. Clips clamp outside their bounds; animation names remain in the source,
  while `source_clips` identifies each imported native clip.
- JOINTS_0/WEIGHTS_0 and optional paired JOINTS_1/WEIGHTS_1: unsigned byte/short
  joint indices; float32 or normalized unsigned byte/short weights. Duplicate
  influences combine; nonzero weights normalize when their total differs from one
  by at most 0.01. Zero totals and larger deviations fail.
- Per-skin inverse-bind matrices retain their supplied float32 values, promoted to
  f64. This profile requires exactly one accessor matrix per joint; extra unused
  matrices are rejected. Missing matrices mean identity. Singular/non-affine matrices fail. A shared
  native rig contains the selected node hierarchy; each skin retains its own palette.
  Mesh-node world transforms cancel in skin evaluation, as required by glTF skinning.
- POSITION-only morph targets, mesh defaults and node overrides, with independently
  animated target weights. Default and key weights must be within [-8,8]; cubic overshoot is diagnosed during evaluation. Morphs precede LBS. Target counts must agree across all
  primitives. [Sparse accessors](gltf_accessors.md) are now supported; normal/tangent
  morph targets remain unsupported.

Approximation consent remains required. Rotation keys whose squared-norm error is
at most 1e-3 are renormalized; the conversion report records the largest source
error. This admits the rounded quaternion samples in the original Khronos SimpleSkin
asset. Larger errors fail. Tangents remain source derivatives. Deformation recomputes
flat geometric normals and discards stale normal/tangent attributes. Existing PBR
roughness and tangent approximations remain reported. Source files are never edited;
callers must retain them for original metadata and source-format round trips.

## Native semantics and persistence

Snapshot v11 adds three explicit absolute TRS channel properties, an optional
per-skin inverse-bind palette and optional default morph weights. Absolute and delta
channels cannot mix on one target; an absolute target requires all three TRS paths.
The existing delta/rest and rig bind-validation semantics remain intact. Empty new
fields are omitted on serialization so older authored snapshots retain their hashes.
Older schema versions reject v11 features, and older readers reject v11 snapshots.
The narrow ABI version is unchanged; these are additive versioned document operations.

The imported scene entities and rig joints are separate native authoring components.
Import duplicates applicable channels to each; subsequent rig/clip edits use the
native animation API rather than treating a scene-entity transform edit as a joint edit.

`merge_animation` appends clips, rigs, skins and morph bindings with collision
rejection. The importer never replaces existing animation state. Ordinary document
transactions provide revision checks, idempotency, permission enforcement and atomic
publication. Saving/reopening native documents preserves the source-to-ID conversion
report returned by the caller only if the caller saves that report; the document
itself preserves all converted authored animation components.

For v11 documents, evaluation without a clip applies the authored default skin/morph
pose. Explicit clip evaluation uses immutable derived meshes and cannot accumulate
pose changes. This default evaluation applies to the entire v11 animation state,
including any earlier native components, so old documents upgraded to v11 should use
explicit clip requests when their intended output is animated. Document versions
through v10 retain their previous no-clip evaluation behavior.

## Limits, failures and backend scope

The importer retains the 4 MiB aggregate source limit, 64 source nodes/materials/
meshes/primitives, 20,000 base points plus corners and 256 transaction commands.
Animation adds at most 32 source clips/skins, 64 joints per skin, 64 morph targets
per primitive, 256 channels/samplers per source clip, 16,384 keys per channel,
131,072 total expanded keys and 131,072 instantiated morph offsets. Native clip
and document validation can impose smaller limits after expansion.

Malformed references, missing resources, duplicate channels, stale topology,
unsupported extensions and singular poses produce structured diagnostics. Skin
joints require a common root. Joints and animation targets outside the selected scene fail; unused skins also
fail. Source metadata cannot execute code or resolve paths. Import conversion is
synchronous and bounded; CLI cancellation/deadline is checked before reads and
publication, not inside every decoder loop. Once a journal commit is admitted it
finishes atomically. Clip sampling, deformation and sequence rendering have
cooperative cancellation. Completed frame artifacts remain explicit on interruption.

| Workflow | Native | Browser local |
|---|---|---|
| Import and exact-time evaluation | Rust | Same Rust/Wasm |
| CPU frames and shutter sequences | Rust CPU | Same Rust/Wasm; existing API resource limits |
| Individual evaluated opaque surfaces | Metal | WebGPU material/geometry subset |
| GPU shutter sequence accumulation | [GPU f32 frames/sequences](gpu_shutter.md) on Metal | WebGPU frames and awaited per-frame consumer |
| Native document round trip | Journal/JSON | OPFS/JSON |
| Animated glTF export, retargeting, drivers, extensions | Unsupported | Unsupported |

This increment advances INPUT-03 and M13 interoperability. M09 remains deferred.
It does not claim arbitrary glTF compatibility or completion of M13.

## Fixtures and verification

[Fixture provenance](../fixtures/animated-gltf/provenance.json) covers the original
synthetic character and unchanged Khronos SimpleSkin bytes (Marco Hutter, CC0).
The synthetic generator writes only a fresh directory. Its test reference computes
joint rotation, root translation and morph offsets independently of the importer.
Tests cover random access, cubic/multiple-target packing, negative/nonuniform scale,
per-skin bindings, default pose, malformed inputs, merging, stale revisions,
cancellation and native persistence. The external source also renders at random times.

`python3 scripts/animated-gltf-workflow.py <fresh-output> --gpu` imports, retries,
evaluates, renders five shutter frames, exports/reopens, compares repeated frames
and exercises faults solely through ordinary CLI requests. It also imports the
external Khronos buffer bundle. The browser client imports the GLB independently,
checks OPFS round trip, random-time frames, conflicts, cancellation and default-pose
CPU/WebGPU comparison. Native/browser frame artifacts are compared numerically.

`python3 scripts/validate-animated-gltf.py` runs the full checks with a local web
server on 8766 and isolated Chrome CDP on 9224. The server serves `web/`; the script
builds current Wasm bindings before browser validation. See the
[evidence index](../evidence/animated-gltf/README.md) for measured results and scope.

Reference: [Khronos glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html),
particularly transforms, skins, morph targets and animation interpolation.

[Sparse accessors and normalized UV0](gltf_accessors.md) now extend the named
static/animated glTF profiles; [acceptance evidence](../evidence/sparse-gltf/README.md)
records equivalent dense/sparse native/browser rendering and recovery.
