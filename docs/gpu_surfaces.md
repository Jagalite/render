# GPU conductor and single-interface coat

The `gpu-f32-conductor-coat-v1` profile renders the existing native-authored typed
conductor and coated surfaces through static, exact-time frame and streaming sequence
APIs. It shares Principled texture, normal, selected UV, vertex color, alpha coverage
and indirect occlusion consumers. [Ideal dielectric transmission](gpu_dielectric.md) has its own later GPU profile.
Sparse media retains an explicit `unsupported_profile` error. This does not add glTF material extensions.

Conductor compiles the CPU f64 eta/k expression into RGB F0, then evaluates Schlick
Fresnel and the existing GGX reflection distribution in f32. Sampling chooses the
GGX lobe exclusively and uses its directional PDF. Color does not replace the
optical constants. Emission remains independently textured.

Coat attenuates the base BRDF by two dielectric Fresnel factors, evaluated for view
and light directions, and adds one weighted GGX interface. Its sampling mixture
chooses the coat with probability weight/2 and the ordinary base otherwise; its PDF
uses the same mixture. The coat-selection random dimension is2+bounce*16; base
selection and direction coordinates use3/4/5+bounce*16. A zero-weight coat preserves
the entire base sampling distribution. IOR1 has zero interface reflectance. This
profile omits repeated scattering between layers and does not model arbitrary stacks.

Private instance row15.w points to two vec4 material records in geometry storage;
zero retains the ordinary base material. Conductor stores a tag plus compiled RGB F0;
coat stores its tag, weight, IOR and roughness. Param9.w selects profile2. These are
private layouts, independent of native surface schemas and durable entity IDs.
An additional32 bytes per extended instance is included in ordinary GPU allocation
budgets. Cache keys include the resulting immutable packed geometry bytes.

A third Rust-generated shader variant is compiled lazily on first use. Opaque and
Principled-alpha variants keep their exact generating output and pipeline behavior.
Each dispatch uses its selected pipeline's bind group layout; destruction releases
all three pipelines. CPU material and transport source is unchanged. No snapshot,
ABI, runtime dependency or foreign computational component is introduced.

Like the other GPU profiles, direction arithmetic, Fresnel and stochastic branch
thresholds use f32. Near-threshold decisions and grazing paths can differ from CPU
f64. Primary textures use camera differentials; secondary textures use the explicit
base-level approximation. Existing finite-depth bias, alpha traversal bounds,
point lighting and lack of MIS remain stated restrictions.

Reproduce using `python3 scripts/validate-gpu-surfaces.py` with the exact browser
package served at localhost:8773 and isolated Chrome CDP at9224. The source recipe is
`fixtures/gpu-surfaces/cases.json`; `render-host kernel --surfaces` emits the generated
artifact. The [evidence record](../evidence/gpu-surfaces/README.md) contains analytic,
CPU/GPU, texture/alpha, shutter/sequence, recovery, failure and resource observations.
