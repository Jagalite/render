# Indexed source attributes and independent fixture validation

The glTF source adapter requires indexed TEXCOORD, COLOR, JOINTS and WEIGHTS names
to start at zero, be consecutive, and use canonical decimal suffixes without
leading zeroes. This validation precedes the existing supported-feature checks.
Unsupported material/attribute families still fail their own profile admission.
Animation input accessors require scalar min/max values matching their decoded
float32 endpoints, in addition to the existing strictly increasing time checks.

Review against the primary glTF specification found that the original named-UV
fixture declared UV0, UV1, UV2 and UV7 with a gap. The morph fixture inherited this
metadata issue and omitted its input accessor's [0,1] time bounds. The independent
Khronos validator confirmed both issues. The corrected fixtures declare UV3..UV6
as aliases of the existing constant UV7 accessor and add the unchanged time bounds.
All external .bin geometry, image and animation bytes remain identical. The
intentional source metadata correction changes glTF/GLB hashes and therefore
source-scoped native entity/revision identities; those identities must not be
silently reused. Canonical native documents from older imports remain loadable.

Analytic color, normal, displacement and deformation values stay unchanged.
Named-UV tests now exercise all eight retained sets and require all seven omitted
sets in the explicitly lossy geometry exporter reports. Negative source tests
cover gaps, missing zero, padded/invalid/overflowing suffixes, and absent/malformed
or incorrect animation time bounds. The native and browser workflows retain
stale/cancelled/invalid/budget behavior and durable recovery.

`scripts/validate-gltf-fixtures.cjs` runs the pinned official Khronos validator as
a development-only independent oracle. It validates embedded GLB resources and
records every diagnostic; it never supplies product import, evaluation or render
behavior. No Rust runtime dependencies change. To reproduce, install
`gltf-validator@2.0.0-dev.3.10` with scripts disabled in a temporary directory and
set `GLTF_VALIDATOR_MODULE` to that installed package path before running the script
with a new output directory. Package identity, license and tool hashes are retained
in its report. No validation diagnostics are suppressed.

The [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
and [Khronos validator](https://github.com/KhronosGroup/glTF-Validator) supply the
external contracts. The source adapter remains a bounded import profile rather
than a complete glTF schema validator. Independent source validation complements
Rust numerical, state and rendering tests; neither replaces the other.
