# Input-to-render support commitments

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

The limited M00–M04 rendering profile is the starting point. The requested target includes complete scene assets, richer materials, animation and rigs, hair, volumes, and native Blender file profiles. These remain staged capabilities. M05 now delivers a bounded static glTF/GLB scene profile and complete restricted-variant workflow; INPUT-01–05 remain partial: M07/M08 supply native authored rendering and animation, while the external-format and wider GPU portions retain their own gates. INPUT-06 remains planned. [Animated glTF/GLB import](animated_gltf.md) now delivers
the named INPUT-03 import/save/reopen/render workflow on native and browser Rust,
with [separate evidence](../evidence/animated-gltf/README.md). Sparse accessors,
normal/tangent morphs, extensions and animated source-format export remain open.

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
