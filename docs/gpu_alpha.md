# GPU Principled alpha coverage

The validated `gpu-f32-principled-alpha-v1` profile admits the existing typed
Principled OPAQUE, MASK and BLEND surfaces on Metal and browser WebGPU. It uses
ordinary immutable evaluated scenes and the existing static/frame/sequence APIs.
No document schema, ABI, runtime dependency or editor mutation path is added.
[Conductor and single-interface coat](gpu_surfaces.md) now have a separate GPU profile.
Dielectric and sparse media remain explicit `unsupported_profile` errors.
[Acceptance evidence](../evidence/gpu-alpha/README.md) records 145 native tests,
124 Wasm tests, real Metal/Chrome workflows and unchanged prior opaque artifacts.

Coverage multiplies base-level decoded texture alpha, barycentric linear vertex
alpha and the material factor. Named UV selection and sampler wrapping/filtering
are shared with the color consumer. OPAQUE ignores alpha. MASK is inclusive;
a bounded binary search over positive f32 bit patterns finds the first alpha
accepted by the CPU f64 multiplication predicate. This preserves decoded constant
thresholds, including division-rounding and factor-underflow cases, in at most
30 comparisons. Zero factors/cutoffs and finite cutoffs above one have explicit exits. BLEND samples stochastic coverage using the extended
CPU counter dimensions. Transparent primary crossings do not consume BSDF depth.

The existing opaque kernel is retained byte for byte. Alpha uses a separate
variant generated from the same Rust IR builders, compiled lazily on first use.
Opaque-only renderers incur no second pipeline compilation. Each bind group uses
its selected pipeline layout. Generated artifacts and both generating inputs are
recorded; the opaque shader hash is checked against commit07720c5.

The private instance row15 stores mode/factor/compiled cutoff. Param9 selects the
extended stride16 schedule; old diffuse/PBR profiles keep their original schedule.
Base-level alpha arrays retain f32 decoding beside the existing RGBA16 color/mip
storage. Texture/vertex interpolation and their product remain f32: coverage near a
threshold can differ from CPU f64, especially under interpolation or GPU subnormal flushing. Zero alpha explicitly
rejects a positive cutoff even if its packed threshold flushes to zero. This is a named
precision profile, not universal pixel identity. The largest f32 RNG midpoint can
round to one; full coverage therefore bypasses the stochastic comparison.

Primary traversal keeps the original ray and camera interval, advancing only the
minimum hit distance. Its depth remains relative to the original per-pixel origin.
Perspective near values below the secondary-ray epsilon are honored in both BVH
and triangle admission; orthographic clipping retains the existing minimum 1e-5m.
Continuation accepts at most 64 discarded surfaces, then returns `budget` on a
65th discard. Deterministic shadow/end-of-depth visibility multiplies (1-alpha)
through at most 64 iterations, with the same early opaque/miss exits as CPU.

A negative private depth marker carries traversal failure through spatial samples,
nominal passes and later shutter dispatches. Full readback translates it to a
structured budget error before any frame is published. A sequence retains only
its previously emitted complete frames. Cancellation remains observable before
submission and after ordered completion; this profile does not claim GPU preemption.

The extended CPU renderer also corrects an occlusion omission: outgoing indirect
throughput now includes sampled occlusion for Principled, coated, conductor and
dielectric models. Current direct illumination and emission remain independent.
Analytic zero/full/partial AO tests reproduce the original error and verify recovery.

Primary PBR color textures use camera differentials; secondary color textures use
zero derivatives and base-level magnification filtering. The subsequent shared
[secondary footprint correction](secondary_textures.md) validates the extended CPU
path against this contract. Small emissive lights, MIS, richer layering, refraction,
fiber shading and participating media retain separate gates.

Reproduce with `python3 scripts/validate-gpu-alpha.py` after serving the exact
`scripts/build-web.sh` output at localhost:8771 and launching the isolated test
Chrome CDP endpoint at localhost:9224. Fixture generation is separate from rendering:
`python3 fixtures/gpu-alpha/generate.py <new-directory>`. Observed images are retained
for comparison; the workflow does not regenerate approved reference images.
