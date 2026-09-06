# Bounded evaluated PBR GLB export increment

Based on vertex-color commit 6c3a969. Architecture sections 21 and M03/M13 require
explicit interoperable profiles and losses, not native-document replacement.
Primary Khronos specification checked 2026-09-06: node matrices must be TRS
representable; normals/tangent XYZ are unit vectors; emissiveFactor is [0,1];
indexed attribute sets start at zero and are consecutive.

Shared Rust core entry consumes an immutable snapshot and typed revision/time,
policy and budget request. It evaluates the requested pose through the existing
Evaluator, exports local evaluated triangle attributes and representable node
world transforms, and returns an embedded-resource GLB plus typed report. CLI and
browser/agent dispatch use this same implementation; no native-only export path.
Persistent output uses the existing fresh-directory publication contract.

Scope: PBR metallic/roughness, all five image roles and samplers, CPU MASK/BLEND,
base/emission factors, double sided, normal/occlusion strengths, UV0..7, authored
normal/tangent frames, linear COLOR_0, mesh sharing when geometry/material agree,
bounded authored node labels, and evaluated static poses of animated/deformed/procedural/curve/displaced surfaces.
Preserve encoded PNG/JPEG bytes. Order file tables by stable entity ID independently of private archive layout.
Retain local geometry so per-vertex direction
interpolation under nonuniform transforms is not altered by flattening directions.
Reject shear world matrices, unsupported BSDFs/legacy textures, non-unit direction
arrays, HDR emission outside core glTF, sparse media and output exceeding the
admitted 4MiB/64-node/64-material/16-image/10000-exported-vertex profile. Explicit
world-position quantization tolerance gates f32 output; reject collapsed triangles.

Loss report: authored hierarchy/topology/IDs/custom attributes, clip/rig/controller
and procedural intent become evaluated triangles; settings, lights, cameras,
environment, non-renderable or hidden authoring data are not a portable scene render
recipe. Native archive remains the lossless source. Report selected time and source
revision, exported entity/node mapping and maximum observed position quantization.
Material UV selectors map stable native IDs into consecutive glTF sets. No silently
removed shading features. Additional geometrical provenance remains in the report.

Evidence: original glTF UV/color/morph/alpha fixtures exported, independently
validated, reimported, persisted and rendered through native CPU/Metal and browser
CPU/WebGPU where supported. Exact semantic checks for shared assets, sampler roles,
selected coordinates and encoded image hashes; numeric pixel/depth/normal tolerances
and source-scoped ID mapping; analytic transform/quantization and rejection tests;
stale/cancel/output-budget atomicity; unchanged original fixtures/runtime deps and
previous render outputs. Failed development attempts retained, no goldens regenerated.
