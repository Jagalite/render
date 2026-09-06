# External HAIR strand input contract

Base 69a4882. Apply architecture 8.2/14/15, M07, INPUT-04, existing curve sweep
and source/provenance contracts. Retain fully Rust runtime, immutable transactions,
native/browser shared semantics and M09 deferral. Read the public format description:
https://www.cemyuksel.com/research/hairmodels/ (retrieved 2026-09-06).
Do not embed cyHairFile/C++ or execute source information bytes.

Admit the 128-byte HAIR header, required XYZ points and optional segment counts,
per-point thickness, RGB and transparency arrays. Explicit policy owns byte order,
meters per unit, radius-versus-diameter thickness, linear-sRGB versus sRGB color,
one-minus-transparency coverage, existing Principled roughness and bounded sweep
representation/tessellation. Source XYZ orientation is retained; no inferred axis,
color encoding, interpolation spline or physical fiber BSDF is claimed.

Preserve root-to-tip polylines, stable source-derived strand/control IDs and varying
positive thickness. This increment admits uniform color/transparency per strand,
including arrays whose values vary between strands. Group identical materials into
native curve assets; reject within-strand varying color/transparency explicitly.
Those require a typed curve-attribute transfer gate, rather than silent averaging,
texture quantization or conversion to uneditable source-less polygons.

Bound input to 4 MiB, strands to 2,048, total controls to 16,384 and material groups
to 128. Retain existing 2..4,096 controls per curve. Preflight actual native sweep
conversion under caller policy and aggregate limits of 32,768 samples, 65,536
vertices and 131,072 triangles. No decimation or fabricated minimum radius. Reject
unknown flags, missing points, inconsistent totals, empty/coincident controls,
invalid fields, truncation/trailing data and derived-budget failures before commit.
Unused default fields remain metadata and are not applied when arrays exist.

Return typed source/policy, original strand/point mapping, asset/material/entity
identities, source bytes and authored/derived resource counts. Use existing geometry
and material transactions and native schemas. Raw source containers/information
bytes remain caller-retained, never commands. Add native project and agent/browser
entry points through the shared cancellable import and durable publication seams.

Require original CC0 default/optional-array/byte-order/color/coverage fixtures,
independent header/data inspection, analytic positions/radii, exact comparison with
manually-authored native curves, numerical conversion bounds and complete CPU/Metal/
Chrome WebGPU/OPFS workflows. Test invalid/unsupported/budget/cancel/stale/permission/
publication failures, archive recovery and source/receipt/shader regression identity.
Only update the named input capability after full workflow acceptance. Physical fiber
scattering, varying strand color, animated groom sources and paging remain next gates.

Resource clarification from native/Chrome evidence: derived f64 JSON length differs
by up to three bytes in this corpus while authored state and rendered pixels match.
Name this field observed_derived_json_bytes and validate it against each platform's
actual published-document evaluation. All source/policy/identity and count fields
still require exact cross-platform agreement; no image tolerance or golden changes.
