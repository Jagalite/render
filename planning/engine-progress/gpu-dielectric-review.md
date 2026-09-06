# Ideal dielectric integration review

Review follows typed authored optics through immutable evaluation, private GPU
packing, generated transport, publication, recovery and client compatibility.

- Existing document validation owns IOR[1,3], metallic0 and roughness0. GPU admission
  now accepts that same type; sparse media remains excluded. No schema, ABI or
  dependency changes. Unsupported-profile tests use a valid native sparse volume.
- Private record tag3 stores IOR in the existing32-byte surface record shape.
  Its one-sided override admits the same exit faces as CPU; it does not mutate the
  authored material. Back orientation follows the established PBR inverse-transpose
  normal policy under reflections. All prior packing remains unchanged otherwise.
- Optics use geometric normals captured before smooth/normal-map shading. Reflection
  keeps unit weight, transmission applies sampled tint×eta², and signed offsets follow
  the actual outgoing hemisphere. The last-depth visibility and alpha failure paths
  are shared with existing extended transport. No point-light delta-BRDF term is added.
- A fourth lazy variant carries all existing surface functions. Actual tests alternate
  dielectric and the other three pipeline classes. The mixed fixture explicitly
  asserts presence of dielectric, conductor, coat, Principled and legacy diffuse;
  an earlier development version contained only two PBR models and was corrected.
- Sixteen geometric/analytic recipes cover optical limits, Snell/TIR, entry/exit,
  mirrored and tilted frames, grazing, clipping and alpha. Native Rust and independent
  Python recipe expectations use documented counter samples and optical factors,
  without consuming rendered reference images. Persisted CLI/browser cases repeat
  those assertions and archive/OPFS recovery alongside textures and morph shutters.
- New native/browser GPU workflows retain cancellation, invalid/stale/budget failures,
  nominal/temporal alpha errors and partial sequence publication. Exact old shader
  hashes and prior rendered artifacts are checked separately from f32 tolerances.
- Limitations remain visible: air/material at each face without nested-medium state,
  ideal smooth interfaces, f32 critical-angle and branch precision, no absorption
  thickness, no rough transmission and no directly sampled glass point-light caustics.

Development logs retain fixture-helper compile errors, selecting a transform-only
root instead of its material-bearing primitive, canonical receipt comparison and the
mixed-model fixture correction. These required no tolerance or golden changes.
