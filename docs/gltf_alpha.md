# glTF alpha through the shared Rust material engine

The `gltf2-alpha-cpu-v1` capability extends the PBR scene importer with MASK and
BLEND. It converts them to the existing typed Principled surface and Mask/Blend
opacity models. Native Rust CPU and browser Rust Wasm share those semantics;
Metal/WebGPU coverage is now validated in the additive [GPU alpha profile](gpu_alpha.md).
OPAQUE keeps its existing CPU/GPU path. This is a bounded input extension, not completion of INPUT-02.

The source baseColorFactor alpha defaults to one and multiplies the linear alpha
channel of the base-color texture. sRGB conversion affects RGB only. MASK accepts
coverage when alpha is greater than or equal to alphaCutoff, default 0.5. Cutoffs
are finite and nonnegative, including values above one, which discard every hit.
BLEND implements stochastic surface coverage: expected radiance follows the over
operation; each finite-sample output has sampling noise. OPAQUE ignores alpha.
Declared cutoff must be numeric/nonnegative and requires an explicit alphaMode;
its value is ignored for BLEND/OPAQUE. Unknown modes and extensions are rejected.

Alpha lookup uses the base mip with the declared sampler. This retains the existing
extended-material approximation; it does not claim derivative-filtered alpha or
transparent refraction. Single-sided surfaces are culled, while double-sided
surfaces reverse their shading normals when viewed from behind. Transparent
continuations retain the existing 64-surface limit and cancellation boundaries.
Depth, normal and object passes describe the first accepted hit of spatial sample
zero; they are not averaged fractional-coverage passes.

Transparent primary continuations now retain the original camera clipping
interval. Moving the origin past a discarded hit reduces the remaining interval
instead of opening an infinite ray. Depth is measured from the original per-pixel
camera ray origin, which corrects off-center orthographic depth. Secondary
scattering rays remain independent of camera near/far planes. An exhausted
primary interval returns the environment without passing reversed bounds to media
transport; the first ray preserves authored near distances below the continuation
epsilon. The reproduced
pre-change native CLI results are retained in the acceptance evidence.

There are no public layout or runtime dependency changes. Existing snapshot-v8
advanced material fields carry alpha. The native Mask validation domain expands
from cutoff [0,1] to finite cutoff >=0, preserving the source threshold exactly;
older engines may reject those newly admitted values and must not silently clamp
them. Existing accepted opacity values retain their interpretation. Transactions,
source-scoped IDs, explicit resource isolation and save/recovery paths are unchanged.

The source contract is the official [glTF material schema](https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/schema/material.schema.json).
The [original fixture](../fixtures/alpha-gltf/README.md) supplies an independent
analytic oracle. [Acceptance evidence](../evidence/alpha-gltf/README.md) records
native/browser workflows, resource use, failures and prior-profile regressions.

Named UV bindings are covered by [their own profile](named_uv.md).
[Vertex colors](vertex_colors.md), [GPU alpha](gpu_alpha.md) and [evaluated GLB
export](gltf_export.md) now have separate evidence. Shader extensions, rough
transmission and broader interchange remain separate gates. M09 stays deferred.
