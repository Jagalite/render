# Bounded GPU sparse absorption/emission contract

Candidate feat/gpu-sparse-media, base35ea08b. Apply architecture sections8/14/15,
M07 media semantics, ADR029/VOL3 and the existing GPU material/device contracts.
This is an implementation gate, not a completed capability claim. M09 stays deferred.

Admit up to64 evaluated occupied cells with zero evaluated scattering and ordinary
opaque PBR surfaces (or no surfaces). Reject unsupported combinations explicitly;
no CPU fallback, silent removal, dense expansion or altered native media authoring.
Read-only typed evaluated cells cross the core/GPU boundary; public persisted
schemas remain independent of private BVH and shader packing.

Use inverse affine transforms into half-open unit cells, retaining unit world-ray
parameter and metric extinction/emission. Camera-relative f32 packing must be finite
and nonsingular; reconstructed unit-cell corners must stay within1e-5m of original
camera-relative corners. This measures packing error, not universal near-parallel
intersection accuracy. Camera-relative corners and inverse components are bounded
by 1e12 in magnitude; nonzero packed inverse components and optical coefficients
must be normal f32 (no device-dependent subnormal flushing). These bounds exclude
cells beyond the finite GPU escape sentinel and unbounded intermediate arithmetic.
Preserve arbitrary accepted affine transforms and overlaps.

Sum clipped extinction lengths for transmittance. Integrate emission through at
most2N+1 ordered event intervals, scanning N cells per interval. Sum active RGB
coefficients and use the analytic Beer integral, with a qualified small-optical-depth
series to avoid cancellation. Empty/background cells remain absent. Require analytic
vacuum/tiny/large/RGB optical-depth, overlapping, sparse and half-open-face tests.

Bound each dispatch to8,388,608 conservative cell visits using checked
width*height*samples*max_depth*N*(2*N+4). Existing memory/device limits still apply.
Record geometry packing and algorithmic limits, actual resource observations and
cancellation behavior; submitted GPU work must drain before cancellation returns.
Do not make an unmeasured performance claim.

A fifth lazy Rust-IR kernel preserves all four media-free generated shaders exactly.
Integrate media to primary/secondary hit or camera far; attenuate point-light shadows
and finite-depth escape shortcuts according to the existing CPU reference. Preserve
camera clipping, progressive sample identity, passes, cache/device recovery, shutter
frames and sequence publication. No public mutation, FFI or Cargo change is planned.

Acceptance requires native/Wasm and analytic tests, Metal/Chrome WebGPU comparisons,
original licensed fixtures including VOL3 import and native/OPFS recovery, invalid
numeric/scattering/material/count/work/byte-budget rejection, stale/cancellation and
sequence failure evidence, source/dependency/shader provenance and previous artifact
identity. Perform a cross-domain integration review before declaring the profile.

Single scattering, alpha-continuation media, advanced surfaces, larger sparse
acceleration/paging and simulation retain separate gates. The CPU profile remains
available for those media configurations it already supports.
