# Static opaque PBR slice evidence

The bounded [gltf2-static-pbr-v0 profile](../../docs/static_pbr.md) passes native
and browser workflows. **M07 and INPUT-01/02 remain partial.** This slice adds
opaque metallic/roughness surfaces, typed PNG/JPEG textures and samplers, normal
maps, and authored perspective/orthographic cameras. It does not deliver alpha
coverage, animation, skins, hair, volumes, arbitrary extensions or a full editor.

All 17 M05 regression gates passed, including **53 native tests and 43 WASM
tests**, strict native/WASM Clippy, Linux/Windows compile checks, ABI v0/v1,
independent Python/C clients, native stream faults, Metal, browser WebGPU and
OPFS recovery. Post-review native/WASM tests and Clippy also passed after the
back-face normal-frame correction. Historical M00–M05 evidence and reference
images were not changed.

| Evidence | Result |
|---|---|
| [Native workflow](workflow_report.json) | Two licensed GLBs imported, saved, reopened exactly and rendered on CPU/Metal; retry and cancellation checked |
| [Browser workflow](browser_workflow_report.json) | Same assets through Rust WASM/WebGPU, exact OPFS/export roundtrips, cancellation and stale revision rejection |
| [Sampler and camera matrix](sampler_camera_report.json) | 54 comparisons: three wraps, six min filters, three camera configurations; far-plane clipping on both lens types |
| [Back-face rendering](back_face_report.json) | Mirrored normal fixture from behind; entire tangent frame reverses |
| [Native/browser comparison](cross_platform_report.json) | Independent PFM/JSON pass comparison; matching visibility and bounded RGB/normal/depth error |
| [Regression commands](regression_report.json), [final checks](validation_report.json), [post-review checks](review_checks.json) | Commands, exit codes, logs, timings and resource measurements |
| [Dependency inventory](dependency_inventory.json) | 106 native and 94 browser packages, enabled features, native links and generated binding/WASM hashes; no denied compute dependencies |
| [Integration review](accepted_ADRs.md) | Versioning, shader layout, precision, failure history and remaining limits |

At 64×64 and 32 samples, CPU/Metal RGB RMSE was **1.56e-5** for BoxTextured and
**0.01034** for NormalTangentMirrorTest. The latter's normal RMSE was **4.19e-5**;
both had **zero object-ID mismatches**. Gates were declared before execution:
RGB/normal RMSE ≤0.025 and ≤4 object-ID mismatches. The 54 sampler/camera cases
had RGB RMSE at most **3.54e-5**. Rear-view RGB/normal RMSE was **5.99e-5/4.32e-5**.
Native Metal versus browser WebGPU RGB RMSE was at most **8.97e-6**, with zero
object-ID mismatches and depth RMSE below **6.5e-8 meters**.

Independent Rust tests cover normal-incidence dielectric/conductor values,
reciprocity, white-furnace energy, hemispherical quadrature, mixture PDFs and
sample means, sRGB conversion before filtering, NPOT mip values, wrap/min/mag
filtering, bounded PNG/JPEG decoding, JPEG import/persistence/rendering, normal
inverse-transposes, tangent handedness under reflection, culling and back-face
frames, clipped cameras, failed/stale/retried transactions and cancellation.
These analytic checks supplement same-implementation backend comparisons.

The final **debug native workflow took 59.87 seconds**, with maximum RSS
**806,207,488 bytes** and peak footprint **992,513,472 bytes** on Apple M1/macOS.
The large fixture retained a 6,772,087-byte authored snapshot and packed
135,337,648 GPU bytes, including a full 128 MiB RGBA16 texture binding. Its CPU
render took 1.58 seconds; GPU packing/submission/readback took 3.96 seconds.
These figures describe this workload, not a speed or portability claim. Browser
WASM was optimized; its timings are not comparable to native debug timings.
Browser tab total memory and physical VRAM were not measured. Linux/Windows
were compile-only checks, with no runtime claim.

`renders/` contains newly measured PPM/PFM images, receipts and diagnostic passes,
including the mirrored fixture's rear view. Visual inspection confirmed visible
textured cubes and the mirrored test grid from both sides. These outputs are
run artifacts, not regenerated reference-image goldens. The upstream fixture's
horizon-environment reference is not a claimed match: this profile uses constant
environment illumination plus a point light.

Source files and fixture bytes are pinned by [source_manifest.json](source_manifest.json).
Run `python3 scripts/validate-static-pbr.py` with the isolated server/browser
setup in the profile guide to reproduce. The M05 checkpoint is `de07c70`; this
candidate is on `feat/static-pbr-m07` and is not committed or pushed.
