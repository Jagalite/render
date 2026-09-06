# Evaluated PBR GLB export

`gltf2-evaluated-pbr-export-v1` exports a pinned evaluated scene through shared
Rust code. A typed request contains `revision`, optional `at: {clip, time}`, and
an explicit policy. Default evaluation uses the existing default-pose evaluator;
`at` exports one exact rational-time pose. The operation never mutates its snapshot.

The profile retains local triangle positions, authored normals/tangents, UV0..UV7,
linear COLOR_0, representable world transforms and matching geometry/material
instances. Geometry arrays are shared even when material assignments require
separate glTF meshes. Native stable UV selectors map into consecutive source sets.
The five PBR texture roles retain their encoded PNG/JPEG bytes, sampling modes,
normal/occlusion strengths, base/emission factors and double-sided behavior.
Core glTF OPAQUE, MASK and BLEND semantics are exported; the latter two retain the
existing CPU-only render qualification. Exporting a material does not extend GPU
scattering support.

Local attributes and transforms remain separate. Transforming and normalizing
individual smooth normals into world space would change their interpolation under
nonuniform scale. The profile therefore rejects world shear, which cannot be
represented by a core glTF TRS matrix. It retains nonuniform and reflected TRS
transforms. It also rejects unsupported BSDFs, legacy diffuse/texture semantics,
HDR emission outside core glTF, non-unit authored frame vectors, varying tangent
handedness within one triangle, collapsed f32 triangles and sparse media. These
limits follow the [Khronos glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
and the engine's separately published adapter/render profiles.

The required policy fields are:

```json
{
  "allow_approximations": true,
  "max_bytes": 4194304,
  "max_vertices": 10000,
  "max_position_error_meters": 0.00001
}
```

`max_bytes` bounds the complete GLB, including JSON, headers and padding; the CLI's
artifact budget additionally includes the report. Bounds cannot exceed 4 MiB,
10,000 exported vertices, 64 surface instances/materials/textures, 16 images and
12 million decoded image pixels. Node labels are bounded to 1024 UTF-8 bytes. Vertices count each distinct mesh/material
combination, matching the bounded import path. Position and transform components
are rounded to f32; export measures the maximum world-space vertex displacement
and rejects values above the requested positive tolerance, which cannot exceed
one meter. This is a positional bound, not an angular or external-renderer image
guarantee. Authored normal/tangent squared lengths must lie within 0.000001 of one.

The report contains authored/evaluated revisions, the optional animation receipt,
source-entity-to-export-node mapping, resource counts, measured positional error,
GLB byte count and content digest. Source indices describe this file only; import
creates new durable IDs and its own source-to-ID report. Node labels preserve
authored entity names. File table ordering uses stable entity IDs, so in-memory
and reopened snapshots with different private chunk order export identical bytes. The exporter reports loss of authored hierarchy, polygon/loose topology,
custom attributes and stable attribute IDs, rig/clip/controller
and procedural intent, hidden/non-renderable authoring objects, cameras, lights,
environment and render settings. Curves and displacement export their evaluated
surface, not authoring semantics. Native document archives remain the lossless path.
The comparison workflow deliberately reapplies the same explicit render recipe.

CLI example content for the existing pinned `export` operation:

```json
{
  "format": "glb",
  "at": null,
  "policy": {
    "allow_approximations": true,
    "max_bytes": 4194304,
    "max_vertices": 10000,
    "max_position_error_meters": 0.00001
  }
}
```

The fresh output directory contains `scene/scene.glb`, `scene/report.json`, and
`manifest.json`. Existing output publication rules apply; stale, cancelled,
unsupported or budget-failed work never publishes a complete scene bundle.
Browser/agent dispatch uses `export_glb` with `request: {revision, at, policy}` and
returns `{glb, report}` from the same core function. The bounded synchronous browser
operation can reject at admission; the core API and native control additionally
check cancellation throughout evaluation/encoding. There is no required service,
filesystem resolver, native-only exporter, schema migration or runtime dependency
change. The API addition leaves existing JSON/ABI operations intact.

The [fixture recipe](../fixtures/gltf-export/README.md) combines original UV,
vertex-color, posed-morph and alpha sources with independent exported-file
validation, render comparisons, persistence, precision and failure oracles.

[Acceptance evidence](../evidence/gltf-export/README.md) records the complete native
and browser workflows, independent format checks and the reproduced/fixed export
ordering defect.

The additive [GPU alpha acceptance](gpu_alpha.md) also renders MASK/BLEND exports
on Metal/WebGPU, including named UV, occlusion, equality and subnormal-factor cases.
Only images referenced by evaluated material bindings are export dependencies;
unused source images retained by import do not need to appear in an evaluated GLB.
