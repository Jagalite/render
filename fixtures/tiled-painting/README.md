# Tiled painting fixture contract

Original CC0-1.0 inputs author a 64 by 64 canvas with two named layers, a checkpointed
pressure-varying ochre stroke, a blue diagonal, a non-destructive mask and an erase.
Use the qualified UV fixture to unwrap and pack a cube, then explicitly bake and
bind this canvas to its named UV set. Save/reopen and render through CPU and GPU on
native and browser. Compare source/tile/image identities and revisions exactly;
GPU comparisons use the established renderer tolerances. Input generation is
reproducible; it does not generate reference images.

`python3 fixtures/tiled-painting/generate.py /tmp/painting-inputs` must reproduce
`data` byte-for-byte. Retain both stroke checkpoints, accepted requests, resource
reports, failure/cancellation diagnostics and old-client rejection evidence. Unit
contracts additionally use independent small alpha/color examples, partial edge
tiles and one-shot versus checkpointed replay. The bounded workflow is qualified in `evidence/tiled-painting`.

`srgb8-native-baseline.json` is a retained numeric compatibility oracle, not a
render reference image. It records all 256 outputs of the unchanged be20d1f native
8-bit sRGB transfer source, plus the 23 codes whose prior Wasm results differed by
one bit. `scripts/generate-srgb8-compat.py` deterministically emits the portable Rust
table from that immutable input; `scripts/validate-srgb8-compat.py` verifies source
provenance and a 64-digit Decimal analytic error bound. Do not regenerate the oracle
or reference images to pass a gate. Changes need an explicitly reviewed profile.

`mip-native-baseline.json` retains the native LOD values for the painted fixture's
1,041 observed primary-hit texture footprints. All mip pixels, UVs and footprints
agreed between native and Wasm, but 144 platform f32 logarithms differed. Shared
Rust f64 log2 rounded to f32 matches every observed native value;
`scripts/validate-paint-mip-lod.py` independently checks Decimal64 rounding. The
public sampler test exercises these weights on analytic constant-color mip levels.
