# Extended surface secondary texture footprint correction

Base1460e14. Preserve fully Rust computational runtime, immutable transactions,
native/browser material semantics, both generated GPU kernels and M09 deferral.
Applicable contracts: docs/multibounce_pbr.md (primary derivatives, secondary LOD0),
docs/gpu_alpha.md and ADR025. This is an identified CPU transport correction.

Reproduce with an original textured emitter visible only after a bounce. With
primary surfaces untextured and secondary LOD0, changing only the emitter's
minification filter cannot change the image. Keep the magnification filter fixed.
An ordinary opaque control must pass before the fix; extended Principled, coated,
conductor and dielectric paths must satisfy the same invariant after correction.
Verify primary minification still operates and that transparent primary crossings
remain primary surfaces for differential selection.

Share the existing PBR shading implementation with zero secondary derivatives,
including stable selected UV attributes, normal/emission/ORM/AO bindings. Do not
change BSDF sampling, alpha, clipping, shader generation or public schemas. Record
the approximation explicitly; this does not implement propagated ray footprints.

Acceptance: native and Wasm positive/negative tests, reproducible CC0 GLBs, ordinary
CLI import/transaction/render/recovery, CPU/Metal and Chrome WebGPU/OPFS workflows,
primary/secondary sampler invariants, cancellation/stale/budget failures, resource
and provenance evidence. Verify both GPU shader hashes and prior rendering artifacts
are unchanged. Update capability documentation only after the complete workflow.
