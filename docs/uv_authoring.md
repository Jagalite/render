# UV authoring

This M10 engine subgate adds named corner UV authoring through `author_uv` in the
project CLI and shared agent API. The [acceptance evidence](../evidence/uv-authoring/README.md) qualifies the bounded
profile; M10 remains incomplete. Painting, sculpting, remeshing, retopology and the M09 interactive
workflow remain separate work.

Requests pin a revision, entity, source mesh and optional source UV asset, and carry
an idempotency key, computation budget and transaction byte allowance. Unwrap and
packing prepare ordinary `put_mesh`, `put_uv_asset`, `set_uv_asset` commands. The
existing publication path enforces authorization, revision checks and durable
all-or-error writes. Retained immutable sources reproduce retry commands even after
later authoring. Other entities sharing a mesh retain the original geometry.

`uv-lscm-disk-v1` uses stable edge seams and corner pins. Uncut edges join corners
into connected charts. Each chart must be an oriented manifold disk with one simple
boundary. Explicit pins require at least two chart vertices at distinct positions;
unpinned charts receive deterministic farthest-pair solver anchors. The independent
Rust implementation solves the conformal least-squares system with column-pivoted
Householder QR. It reports residuals and a pivot ratio, not a condition number.

The stored f32 result must have positive triangle areas, no interior triangle
overlap and no nonincident boundary contact within the profile's tolerance. Source
positions, topology, durable element IDs and other attributes remain unchanged.
Authored tangent/tangent-sign attributes require explicit resolution before UV
changes; generated frames are recomputed by existing evaluation. Adding a UV name
freezes the former effective default attribute ID so existing texture bindings keep
using their coordinates.

`uv-rectangle-atlas-v1` places the largest rectangles first, with stable face-ID
ordering for ties and bottom-left edge candidates. It does not rotate charts or
promise optimal packing. Resolution, pixel padding, area-average texel density and
pin behavior are explicit. Density and chart areas use authored mesh positions,
before entity transforms. Preserved pinned charts must already fit and have the
requested density. `transform_with_chart` explicitly moves both coordinates and
pin intent. A one-pixel guard supplements requested padding before f32 storage.
Reports distinguish allocated rectangle pixels from actual UV surface occupancy.

The mesh profile allows 65,536 points/edges, 4,096 corners, 1,024 faces and 256
corners per polygon. There are at most 64 charts, each with 128 chart vertices and
256 triangles. Dense QR has at most 256 columns and 512 rows; summed rows times
columns squared is limited to 16,777,216 work units. Packing tests at most 262,144
candidate positions. Atlases have dimensions 1..4,096, padding 0..32 and positive
texel density no greater than 65,536 pixels per meter. UV coordinates are finite
and bounded by 1,024 in magnitude. Mesh plus complete UV asset JSON is limited to
32 MiB, independently of the transaction allowance. These are admission limits,
not claims about process memory or interactive latency.

Snapshot 17 adds typed immutable UV assets and entity bindings. Each asset retains
up to eight named layouts with mesh/attribute identity, seams, pins and optional
atlas goals; at most 64 assets may be retained in a snapshot. Updating one set
carries every other set's constraints forward. Geometry edits must explicitly
clear or replace a binding that would become stale. Evaluated deformation removes
the binding only from its disposable snapshot; authored constraints remain stored.

The optional explicit default UV ID also requires snapshot 17. Mesh chunks use
`R3DMESH1` when this field is present; old meshes retain `R3DMESH0` bytes. Empty UV
tables and absent defaults are omitted to preserve old snapshot hashes. Clients
that support only older versions must reject new state. No Cargo dependency or
computational FFI is added.

Native/core callbacks check cancellation during bounded work and before publication.
The browser authoring call is synchronous and has bounded admission; it does not
claim that a browser event can interrupt a synchronous solve. GPU previews retain
their existing asynchronous cancellation. Future long editor gestures need their
own job/checkpoint contract.

Reproduction inputs are in `fixtures/uv-authoring`. Native and browser acceptance
scripts are `scripts/uv-authoring-workflow.py` and
`scripts/uv-authoring-browser.mjs`; JSON shape validation is separate from Rust
semantic checks. Historical archive recovery preserves the authored states; this
increment does not add an interactive undo command.

Evaluated UV tables put the explicit default first, preserving existing GLB export
and unqualified texture semantics. OBJ writes that default and reports the omitted
named sets. Legacy meshes keep their former name ordering. Packing reports each
chart's affine UV scale/offset and whether its pins were preserved; final stored
coordinates and pins remain the authoritative asset values.
