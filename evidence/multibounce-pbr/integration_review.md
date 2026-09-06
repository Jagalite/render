# Integration review: bounded opaque transport

The existing typed render settings remain authoritative. No snapshot mutation or
version migration is needed; settings edits use ordinary transactions. The agent
budget gate and public JSON settings schema both admit the validated 1–16 range.
The native CLI already forwards typed settings and preserves revision, cancellation
and durable artifact boundaries. Imported scene semantics are unchanged.

CPU and GPU use the same full diffuse/GGX mixture density and vertex-indexed random
dimensions. No emission/environment next-event sampler was added, so BSDF-hit light
is counted once. The point light is discrete and evaluated once at each vertex.
Last-vertex environment escape preserves the previous depth-one semantics; hitting
another surface at the depth cap terminates without shading it. The known bias and
variance limits are documented. Authored occlusion affects only indirect transport.

The private GPU parameter buffer uses previously reserved fields for depth and
sample stride; it is generated and consumed within the current implementation,
not exposed as a durable ABI. Kernel behavior is maintained as Rust IR and Naga
validates generated WGSL. GPU geometry remains disposable and content-keyed. No
extra per-depth buffer or recursion was introduced. Receipts name depth and material
semantics; progressive reset uses the existing complete scene/settings key.

Primary clipping and structured passes are gated on bounce zero. Secondary texture
sampling explicitly uses zero derivatives; it cannot incorrectly project original
camera differentials onto a later surface. The original roughness regularization,
normal mapping and geometric visibility policies remain explicit limitations.

CPU cancellation adds a check at each vertex. GPU work remains bounded per dispatch
and cancellation is checked before submission and after completion; the acceptance
test invokes the existing real queue-submission fault hook. Tests require stale
revision failure and immutable authored state through native and browser clients.
Linux and Windows compile coverage must not be described as runtime validation.

Numerical acceptance is independent of backend parity: diffuse finite-series values
and a closed-form rough-metal hemispherical integral constrain transport. Existing
BRDF quadrature/energy/reciprocity tests remain unchanged. One historical test's
unsupported depth-two assertion is replaced by invalid depth 17, with depth-two
success now covered by dedicated analytic and CLI/GPU workflows. The CLI regression
now exercises depth two directly. Historical evidence and reference images remain
untouched. The explicit adapter-dependent GPU test is run as a mandatory acceptance
gate even though ordinary CPU-only test invocations omit it.

No Cargo manifest, lockfile or computational dependency was changed. The new plate
fixture is original CC0 data and its generator only writes a new output directory.
The secondary texture test reuses the unchanged, licensed Khronos Cesium texture.
