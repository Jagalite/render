# Shared secondary texture footprints

The extended Rust CPU path now uses zero UV derivatives after a scattering bounce,
matching the existing opaque CPU and Metal/WebGPU approximation. This applies to
Principled, coated, conductor and dielectric surfaces through the shared PBR shading
function. Primary surfaces retain camera differentials, including surfaces reached
through discarded primary alpha coverage. Texture binding IDs, samplers, alpha
coverage, BSDF sampling and public schemas are unchanged.

Zero derivatives select the base level and magnification filter. This is a bounded
approximation, not propagated ray differentials or ray cones. It prevents the old
extended path from incorrectly projecting camera footprints onto secondary hits.
[Acceptance evidence](../evidence/secondary-textures/README.md) includes a reproduced
failure: changing only a secondary emitter's minification filter changed CPU pixels
by up to 0.13204649. The ordinary opaque control already satisfied the invariant.

The original CC0 fixture has an untextured floor and a checker-textured emissive
ceiling visible only after scattering. All five CPU surface cases now produce exact
nearest/mipmap image and pass identity. Separate primary-emitter and transparent
primary tests prove minification remains active where camera footprints apply.
A 65-call persisted CLI workflow and ten Chrome/OPFS cases cover native-authored
materials, archive recovery, stale revisions, cancellation and output budgets.
Metal and WebGPU validate the two supported GPU models; richer GPU models remain
explicitly unsupported. Both existing generated shaders remain byte-identical.

Reproduce with `python3 scripts/validate-secondary-textures.py`, with the exact
packaged browser output served at localhost:8772 and isolated Chrome CDP at9224.
Generate fixture source independently with
`python3 fixtures/secondary-textures/generate.py <new-directory>`.
M09 remains deferred; richer GPU scattering is the next transport increment.
