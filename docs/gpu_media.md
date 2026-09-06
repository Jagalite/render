# Bounded GPU sparse media

The validated `gpu-f32-sparse-medium-v1` profile integrates RGB absorption and
emission through overlapping affine sparse cells. Acceptance is recorded in
[the evidence](../evidence/gpu-media/README.md), under
[the contract](../planning/engine-progress/gpu-media-contract.md).

The profile admits 1–64 evaluated occupied cells with exactly zero evaluated
scattering, and either no surfaces or ordinary opaque PBR surfaces. Legacy diffuse,
alpha, explicitly extended Principled, conductor, coat and dielectric combinations
are rejected with `unsupported_profile`. Empty media uses the existing surface
profile. Native authoring and VOL3 import remain sparse and unchanged; there is no
CPU fallback, dense texture conversion, external runtime or simulation claim.

A world ray retains its metric parameter when transformed into each half-open
unit cell. Translated lower/upper bounds are packed separately from the inverse
linear ray map so camera jitter does not round outside both neighboring cells.
Parallel rays on the high face are outside. Transmittance sums clipped
RGB optical depths. Emission walks at most 2N+1 ordered cell-boundary intervals,
summing active extinction and emission before analytic integration. For optical
depth below 0.1, a degree-four normalized Beer integral avoids cancellation; the
alternating-series remainder is at most 0.1^5/720, about 1.4e-8 before f32 rounding.
Vacuum and zero-density emitting cells do not divide by zero.

Each dispatch requires checked
`width × height × samples × max_depth × N × (2N + 4) <= 8,388,608`.
This conservative cell-visit bound covers emission scans, segment attenuation,
point-light shadows and finite-depth escape attenuation. Actual device allocation
and existing byte limits also apply. It is a bounded direct scan, not a sparse
acceleration or paging implementation.

Inverse affine transforms are packed relative to the camera. Reconstructed cell
corners must differ from the f64 reference by at most 1e-5 world meters. Both corner
coordinates and inverse components have magnitude at most 1e12. Nonzero packed
inverse components and optical coefficients must be normal f32, excluding underflow
and device-dependent subnormal flushing. These are admission and packing bounds;
they do not establish a universal error bound for grazing or nearly parallel rays.

The fifth lazy shader is generated from maintained typed Rust IR by
`render-host kernel --media`. All four prior generated shaders must remain byte
identical. Evaluated typed cell access is read-only; no persisted schema, mutation
API, Cargo dependency or FFI shape changes. Media data participates in the existing
content-keyed GPU buffer cache and device recovery lifecycle.

Primary and secondary segments clip to the next surface or camera far bound.
Point-light shadows use the CPU reference's offset interval. The existing finite
path-depth shortcut attenuates escaping environment light and does not append a
new emission tail. This preserves the named finite-depth policy; it does not add
MIS, phase sampling, indirect in-scattering or an unbiased infinite path estimator.

The original CC0 corpus in `fixtures/gpu-media` supplies independently calculated
vacuum, tiny/large optical depth, series boundary, RGB, overlap ordering, sparse
holes, negative scale, shear, near/far clipping and inside-camera cases. Tests also
cover exact GPU boundary rays, opaque surfaces and shadows, bounce depths, maximum
cell/work counts, progressive sample identity, cancellation at queue submission,
device recovery, numerical rejection, stale revisions and failed sequence frames.
CLI and browser scripts exercise native archives, external VOL3 and OPFS recovery.
Resource observations and final packaged-platform results belong in the acceptance
evidence; compilation alone does not qualify this profile.

GPU sparse media acceptance: 167 native and 142 Wasm tests, 376 media CLI calls over 21 cases, 117 external VOL3 CLI calls and 13 shutter/sequence calls, with actual Chrome CPU/WebGPU/OPFS recovery. All 993 prior images/passes and 331 receipts remain byte identical, as do all four prior shaders and Cargo files. Maximum browser GPU analytic channel error is 3.24428673e-07. The named profile admits 64 zero-scattering cells and ordinary opaque PBR surfaces under explicit work/precision limits. Scattering, joint advanced surfaces and larger paging remain separate gates; M09 stays deferred.

Reproduce the full acceptance with `python3 scripts/validate-gpu-media.py`, serving
this checkout at port 8778 with the isolated Chrome CDP endpoint at 9224. Linux and
Windows checks establish compilation only; actual runtime evidence is macOS ARM64
Metal and Chrome WebGPU/Wasm. Explicit device destruction/recreation was tested;
uncontrolled driver loss was not injected.
