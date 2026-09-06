# Input-to-render support commitments

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

The limited M00–M04 rendering profile is the starting point. The requested target includes complete scene assets, richer materials, animation and rigs, hair, volumes, and native Blender file profiles. These remain staged capabilities. M05 now delivers a bounded static glTF/GLB scene profile and complete restricted-variant workflow; INPUT-01–05 remain partial: M07/M08 supply native authored rendering and animation, while the external-format and wider GPU portions retain their own gates. INPUT-06 remains planned. [Animated glTF/GLB import](animated_gltf.md) now delivers
the named INPUT-03 import/save/reopen/render workflow on native and browser Rust,
with [separate evidence](../evidence/animated-gltf/README.md). [Normal/tangent morphs](morph_frames.md) now have their own authored-frame profile;
extensions and animated source-format export remain open.

The machine-readable work and acceptance criteria are in [planning/input_support.json](../planning/input_support.json). Existing milestone dependencies remain in force. M05 proves the agent workflow and narrow ABI before the broader product gates. Its [explicit profile](agent_alpha.md#static-scene-profile) does not claim cameras, textures, complete PBR or arbitrary glTF compatibility.

| Track | User-visible outcome | Milestone ownership |
|---|---|---|
| INPUT-01 | Import a complete static glTF/GLB scene, preserving objects, hierarchy, instances, cameras, geometry attributes and material assignments | M05 integration / M13 interchange |
| INPUT-02 | Render supported PBR materials, texture roles, normal maps, alpha and richer surface scattering on CPU and GPU | M07 |
| INPUT-03 | Import and render animated characters with clips, skinning and morph targets at requested times | M08 |
| INPUT-04 | Save, reload and render authored curves/hair with supported strand geometry and scattering | M07; animated deformation in M08 |
| INPUT-05 | Import and render sparse density/emission fields with validated volumetric transport | M07; simulation remains M11 |
| INPUT-06 | Read and render versioned `.blend` profiles using native Rust implementations | M13, integrating each required native evaluator |

## Delivery order

First establish complete static scene ingestion and core PBR rendering. They supply shared foundations for both animated glTF assets and useful static `.blend` profiles. Extend animation/rigging, hair and volumes through their native evaluation and rendering gates. Expand Blender compatibility as those evaluators become available. Each increment must produce a usable import-to-render workflow.

Supporting a file extension means publishing its tested versions, features and limits. Imported data must pass document validation, persistence and rendering tests. Preserved but unevaluated data must be identified as such. Required unsupported features produce diagnostics; any allowed approximation or loss must be explicit.

## Acceptance policy

Every track includes an external or independently authored fixture with provenance, import/save/reopen/render coverage, numerical or structural assertions, invalid-input and cancellation cases, resource measurements, dependency audit, and native/browser capability entries. Maintain the original input and a conversion report. Reference-image changes require independent review under AGENTS.md.

Native and browser implementations share authored semantics and Rust-owned computation. No Blender/Cycles process, conversion service, non-Rust shading/geometry library, or required remote renderer may implement these capabilities. Embedded scripts and arbitrary add-ons are not executed. Existing `.blend` limits in architecture section 21.2 remain applicable: selected versions and native feature semantics expand through measured profiles.

The core glTF target is grounded in the [Khronos glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html), including scene hierarchy, accessors, materials, animations and the GLB container. Extensions require their own declared support and tests.

[GPU shutter frames and streaming sequences](gpu_shutter.md) now extend the
opaque animation profile on Metal/WebGPU, with explicit nominal-time passes and
partial-output semantics. Format coverage remains separately qualified.

[Sparse accessors and normalized UV0](gltf_accessors.md) now extend the named
static/animated glTF profiles; [acceptance evidence](../evidence/sparse-gltf/README.md)
records equivalent dense/sparse native/browser rendering and recovery.

[glTF MASK/BLEND alpha](gltf_alpha.md) now imports into the existing Rust CPU/Wasm
coverage profile, with corrected camera continuation and exact native/browser
passes. [Evidence](../evidence/alpha-gltf/README.md) retains analytic and complete
workflow checks. [GPU Principled alpha](gpu_alpha.md) now covers Metal/WebGPU
static frames, shutter accumulation and partial sequence failure semantics.

[Named UV bindings](named_uv.md) support per-role UV0 through UV7 on CPU and GPU,
including displacement preservation and CPU alpha/bake consumers. The
[acceptance evidence](../evidence/named-uv/README.md) records analytic samples,
CLI/browser recovery and unchanged prior render artifacts.

[Authored morph frames](morph_frames.md) support normal/tangent deltas before
affine skinning, including dense/sparse inputs and snapshot-v13 point/corner
correspondence. [Evidence](../evidence/morph-frames/README.md) covers native and
browser frames, shutters, persistence and numerical direction checks.

[Source conformance](gltf_source_conformance.md) now checks indexed attribute
continuity and animation-input bounds. All 13 GLB fixtures pass the independent
Khronos validator; [evidence](../evidence/indexed-attributes/README.md) records
metadata corrections, preserved render values and expected source identity changes.

[Linear vertex colors](vertex_colors.md) now support COLOR_0 across CPU and GPU
PBR rendering, with CPU MASK/BLEND coverage, displacement transfer and albedo
baking. All 18 current GLB fixtures pass independent source validation; the
[acceptance evidence](../evidence/vertex-colors/README.md) records five source
encodings, native/browser recovery and unchanged regression artifacts.

[Evaluated PBR GLB export](gltf_export.md) now carries local geometry frames, UVs,
colors, transforms, encoded textures and supported material semantics through the
same Rust operation on native and browser. [Evidence](../evidence/gltf-export/README.md)
records exact file bytes across persistence/platforms, independent format validation
and render round trips. Authored animation and full scene-render recipes remain
separate export gates.

Native-authored [conductor and single-interface coat](gpu_surfaces.md) now share
Metal/WebGPU texture, alpha and shutter transport. This extends authored rendering;
external glTF material extensions retain a separate input gate.

Native-authored [ideal dielectric interfaces](gpu_dielectric.md) now render on
Metal/WebGPU with geometric Fresnel/Snell/TIR and bounded alpha/shutter semantics.
Nested media, rough transmission and external transmission extensions remain separate gates.

[VOL3 density/emission input](volume_import.md) now decodes external scalar and
aligned RGB grids under explicit cell, metric and optical policy. Native and
browser CPU imports, rendering and recovery share exact corpus results in
[acceptance evidence](../evidence/volume-import/README.md). Sparse paging, other
formats and GPU media transport retain separate INPUT-05 gates.

[HAIR strand input](hair_import.md) now preserves external polylines, varying
thickness and uniform-per-strand color/coverage under explicit interpretation.
Native curves render on CPU/Metal/WebGPU and recover through archives/OPFS in
[acceptance evidence](../evidence/hair-import/README.md). Per-control color, physical
fiber scattering and animated source grooms retain separate INPUT-04 gates.
