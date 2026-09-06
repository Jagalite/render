# Original named-UV GPU alpha fixtures

`generate.py <new-directory>` derives five CC0 material recipes from the original
named-UV quad and PNGs, preserving all source .bin bytes. Provenance records both
input hashes and every emitted file hash. No renderer or reference image is used.

`named-mask` and `named-blend` sample coverage from UV1=(y,x), using decoded alpha
0,85/255,170/255,1. MASK cutoff is .5; BLEND factor is .5. Existing emission and
occlusion bindings retain independent UV selections. `ao-mask` has full coverage,
base color .5, metallic0, roughness.5, no emission/normal map and texture2's red
channel at UV2 for indirect occlusion. Direct illumination is unaffected by AO.

The shared export workflow imports, saves, reopens, exports/reimports and renders
each recipe on CPU, Metal and browser WebGPU/OPFS. Host alpha tests additionally
create native transactional stacks of 63–65 quads, shadow products, animated failure
cases and mixed diffuse/PBR bounces. Native stack transactions intentionally exceed
the separate 64-node glTF importer profile without changing that source limit.

Coverage boundary tests use decoded f32 alpha, inclusive cutoff and a constructed
RNG endpoint: seed311516010, pixel0, sample0, dimension1024 hashes to0xffffff00.
Its CPU midpoint is0.9999999701976776; f32 rounds to1. Full coverage must remain
covered. No randomized search or probabilistic failure is needed to reproduce it.

`boundary-mask` reproduces exact cutoff equality with factor.0031 and decoded
85/255 alpha. `subnormal-mask` uses the smallest positive f64 factor/cutoff; CPU
multiplication accepts alpha above.5, despite cutoff/factor being1. Both retain
the original binary source and exercise Rust native/browser packing and export.
