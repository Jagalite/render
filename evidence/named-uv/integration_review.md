# Integration review

The reviewed source joins typed texture bindings, source conversion, transactional
snapshot validation, derived displacement, renderer geometry, CPU sampling,
Rust-generated GPU packing/shading, alpha and bake consumers. Stable authored IDs
are translated into private dense GPU slots; none are persisted as runtime indices.
Version 12 prevents older snapshots from carrying selectors unnoticed. Optional
omission and boxed command payloads preserve canonical JSON for old inputs.

Role selection is independent for base color, emission, normal, occlusion and
metallic/roughness. The normal fallback uses the selected set's tangent basis and
handedness. Primary mip footprints derive from the chosen coordinates; secondary
and alpha base-mip approximations remain explicit. Named bake sampling transforms
its source footprint through the atlas Jacobian, without changing destination UV
selection. Displacement preserves names/IDs and all sets under its new policy;
its derived receipts and geometry identity intentionally change. Existing authored
geometry is immutable and the source+displacement cache key remains sufficient.

Validated slot bounds precede render/pack, missing required mesh bindings reject
transactions/imports, and at most eight sets contribute bounded triangle payloads.
GPU f32 addressing and device byte admission remain in effect. Explicit consumer
cancellation and malformed/stale operation workflows pass on native and browser.
The full prior default-profile pixel/pass/receipt comparison passed exactly.

Review identified that geometry-only OBJ/glTF export previously described only
non-UV omissions. It now reports every omitted additional UV set by name and ID;
a regression test constrains both adapters. Full material export is not claimed.
No unresolved blocker remains for this bounded named-UV profile.
