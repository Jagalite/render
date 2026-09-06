# M06 prerequisite implementation contract

M08 depends on the M06 authoring milestone. This contract records the bounded operator
profiles validated in [the milestone evidence](../evidence/m06-m08/README.md).

The initial coherent set is polygon primitives; single-face normal extrusion and
convex planar inset; equal-length boundary bridging; edge split and coplanar edge
dissolve; explicit point weld; convex-box bevel; mirror/array; manifold quad
Catmull-Clark subdivision; open-surface solidify; and axis-aligned-box Boolean.
Restrictions are public preconditions, never fallbacks to a different operator.
General curved-surface Boolean, remeshing and arbitrary bevel remain unsupported.

Operations run on exclusive candidates through the existing radial edit view and
validate the resulting canonical mesh before transactional publication. The first
implementation rebuilds canonical arrays/edit connectivity and reports that global
cost. Geometry-local IDs remain durable selections; array indices are private.
Every output reports preserved, split/merged, created/deleted and ambiguous element
correspondence. Interpolated point/corner attributes follow their declared transfer
policy; categorical disagreements fail. UVs remain corner data; new side charts
have an explicit profile. Unsupported domains/transfer combinations are diagnosed.

Direct operations and modifier graphs call the same implementations. Fields have
explicit point domains and scalar/vector types; groups have typed geometry input
and output. Cycles report node identities; group nesting, mesh growth, allocations
and work have hard limits. A procedural asset remains authored data until an
explicit bake command produces a polygon mesh suitable for the initial skin/morph
profile. Save/reopen and direct-vs-graph equivalence are required gates.

New warped quads (for example after Catmull-Clark or point welding) use an explicit
0-2 diagonal realization into triangles, with corner/source maps retained. Original
input polygons still require planarity. The quad Catmull-Clark operator rejects a
triangulated previous result; retain the authored quad cage when changing its
refinement. This is one named refinement profile, not unlimited subdivision-surface
parity. Point welds can change adjacent surface shape; no self-intersection removal
is implied. Box Boolean requires un-attributed axis-aligned box input and reports
new cut-element correspondence as unknown rather than guessing source identities.

## Per-operator contract

Every input first passes canonical mesh, planarity and surface-element checks.
Selections use stable local IDs; missing IDs return `stale_selection`. Output
canonical/radial validation is mandatory and may reject degeneracy caused by an
otherwise valid parameter. No operator removes intersections automatically.

| Operator | Input/degeneracy policy | Correspondence and attributes |
|---|---|---|
| Box | Finite strictly ordered metric bounds | New local IDs, outward quads, no implicit attributes |
| Triangulate | Supported planar simple polygons | Existing points; face/corner source maps; new diagonals |
| ExtrudeFace | One face, finite nonzero signed normal distance | Duplicated point sources; original face sources on cap/sides; each side gets an explicit unit-square UV chart |
| InsetFace | Strictly convex planar face, fraction strictly between 0 and 1 toward vertex centroid | Inner points/corners interpolate toward centroid; ring and inner faces map to the selected face |
| Bridge | Two disjoint equal-length boundary loops with the required opposite orientation | Existing point IDs; new faces and explicit unit-square side UV charts; adjacent source corner weights |
| SplitEdge | Existing edge, fraction strictly between 0 and 1 | New point interpolates endpoints; each incident corner interpolates its own seam data |
| Weld | At least two distinct selected points; average position; self-touching faces rejected | Merged point weights; collapsed faces removed; surviving corner seams retained; warped new quads use diagonal 0-2 |
| DissolveEdge | Interior edge with two coplanar equally oriented faces and a simple joined boundary | Joined face blends source face data; surviving boundary corner data retained |
| Mirror | Axis 0–2, finite plane offset; duplicates the whole source | Source maps on duplicates, reversed winding/corner order; no implicit seam weld |
| Array | 1–128 copies with bounded finite translation step | Each copy maps to source elements; no instance compaction or automatic merge |
| Subdivide | Manifold quad mesh; boundary rule plus Catmull-Clark interior weights | Weighted point/edge/face-derived vertices and bilinear corner data; warped output quads are realized explicitly |
| Solidify | Open manifold surface, valid finite thickness and well-defined averaged normals | Front/back source maps, reversed back winding, explicit side UV charts; no collision correction |
| BevelBox | Outward un-attributed axis-aligned box, positive trim below half extents | Convex 26-halfspace construction; new points associated with source corners, new faces without invented source identity |
| BooleanBox | Outward un-attributed axis-aligned source box and second box bounds; union/intersection/difference | Exact occupied coordinate-cell boundary; empty result is an explicit error; known original point identity retained, new cut relations unknown |

For retained point/corner/face attributes, Linear uses correspondence weights,
Nearest resolves the largest weight deterministically, Normalize renormalizes the
transferred vector, and Categorical rejects mixed values. Newly created edge data
has no implicit default and is rejected when the source carries edge attributes.
Normal/tangent frames are derived and recomputed. Receipts distinguish identity
preservation from geometric source relationships: a created point can also have
merged source weights. They report created/deleted, preserved, split/merged and
ambiguous IDs per domain.

Graph groups expose typed geometry input/output; point fields expose scalar/vector
values and explicit domain. The first fields are position, constants, vector add,
scale, component and sine. Groups permit 256 nodes, nesting 8, at most 32 group
assets and 4,096 expanded execution nodes. Field work is capped at 16 million
node-point evaluations. Mesh profiles cap vertices/faces, serialized output bytes,
262,144 corners and 1,024 corners per polygon. Retained graph geometry has a separate
byte estimate; it is not an allocator peak-memory guarantee. Local split and wide
array runs measure actual global-rebuild cost at 1, 8 and 32 source boxes.
