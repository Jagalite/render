# Polyline RGBA integration review

The interface joins native curve/groom authoring, snapshot admission, HAIR source
interpretation, mesh attributes and existing surface/render/export consumers.

- Optional typed f32 RGBA is validated before evaluation/publication. Missing fields
  retain old canonical bytes; snapshot15 guards both stored assets and groom guides.
  Stable controls and attribute IDs remain independent of private Rust array layouts.
- Only polylines admit colors. Existing homogeneous subdivision carries weighted
  RGBA alongside unchanged geometric components, with finite bounds and f32 output.
  No geometric error claim is repurposed as a higher-order color error guarantee.
- Tube rings and caps have explicit color assignment. White padding covers mixed
  uncolored curves/guides. Child generation clones controls, so root transforms and
  geometric offsets preserve color values through the merged mesh.
- HAIR opt-in identity includes its policy/profile. Source RGB validation and explicit
  color conversion precede storage. Alpha subtraction's rounding is measured in a
  typed report. Opaque/BLEND grouping preserves semantic coverage without averaging.
- Fully opaque new materials use canonical ordinary PBR, matching evaluated GLB
  import's sampler. The development round-trip failure exposed the distinction from
  explicit extended opaque PBR; the old input profile remains unchanged. The original
  image tolerance is retained, and all seven native/browser roundtrips now satisfy it.
- Ordinary transactions and existing durable import admission own permissions,
  idempotency, stale/cancel/budget checks and failed publication. No new command,
  direct editor mutation, FFI shape, runtime dependency or shader is needed.
- Analytic control/ring/cap tests, invalid data and version recovery tests, actual
  native/browser HAIR and authored groom workflows, independent GLB validation and
  original artifact identity jointly establish the bounded integration. Compilation
  or this review alone does not establish completion; final acceptance records do.

Physical fiber scattering, colored higher-order splines, analytic curve intersection,
source-animated grooms and sparse GPU media retain separate contracts and evidence.
