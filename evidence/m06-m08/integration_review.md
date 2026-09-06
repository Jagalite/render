# Integration review

Review covered the typed document/API boundary, derived geometry caches, shading
pack layout, deformation attachments, image products, persistence and capability
claims. No blocking findings remain within the declared profiles after the final
87-native/77-Wasm and workflow run. This is a local implementation review, not an
independent reference-image approval or external security audit. No existing
reference images were changed.

Resolved findings:

1. Quaternion layer overrides could evade the v6 gate applied to entity transforms.
   Snapshot validation and PutLayer now use the same gate; positive transaction and
   rejected older-version tests cover the boundary.
2. Extended root rendering did not forward cancellation through evaluation. It now
   passes the callback to the evaluator before transport; stale/cancel regressions
   confirm typed failure.
3. Displacement could omit loose source elements. Its surface profile now rejects
   them, checks cancellation before evaluation and preserves the authored source.
4. Displacement added a sixth image binding to material validation. GPU packing
   explicitly retains five shading descriptors and the established instance stride;
   height is a geometry-evaluation input. Full external textured PBR regressions pass.
5. Hidden groom anchors were conflated with absent dependency geometry. Composition
   now separates visibility from dependency availability; hidden/deformed-anchor
   tests and native/browser groom workflows pass.
6. Nonplanar skinned quad realization was ambiguous. Skin/morph binding now requires
   explicit triangulation through M06 before binding; authored source and modeling
   correspondence remain durable. Native/browser random-time character sequences pass.
7. JSON stable local-ID maps under tagged commands required canonical decimal key
   parsing. The local map serializer preserves durable identities without Rust ABI
   exposure; transaction and recovered character tests exercise it.
8. Inset corner interpolation now follows the same centroid weights as the inner
   geometry. Modeling reports preserve seam/correspondence policy; new unknown edge
   data fails explicitly rather than receiving fabricated defaults.
9. Box-only bevel/Boolean now reject inward shells. Modeling checks corner/polygon
   work limits before triangulation. Both are covered by the final modeling suite.
10. World-normal bakes now account for reflected winding. A numerical regression
    checks reflected normals; existing atlas overlap/cancellation/revision tests pass.
11. The imaging albedo pass uses the diffuse path for diffuse materials and the PBR
    path only where present. Native/browser view, display, albedo and bake outputs
    compare identically for the imaging fixture.

Intentional limits remain explicit in [the contracts](../../docs/m07_m08.md):
finite depth and single-scattering approximations; geometry sweeps for hair; bounded
box CSG/bevel; global modeling array rebuilds; point-only fields; spatial denoising;
CPU-only extended transport/imaging; nominal-time auxiliary shutter passes; and
compile-only Linux/Windows evidence. Resource reports distinguish serialized output,
retained estimates and process RSS. No unmeasured speed or universal compatibility
claim is made.
