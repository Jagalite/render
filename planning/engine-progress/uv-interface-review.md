# UV authoring interface review before implementation

Candidate feat/uv-authoring starts from 9f7b4bd and must integrate the qualified
static Blender increment before final compilation and acceptance. No source from
that frozen candidate is edited here. Architecture 17.3 and M10 apply; M09 remains
deferred. The first UV engine subgate is not full M10 completion.

Use an immutable typed UV layout asset referencing an authored mesh content hash
and named corner Vec2 attribute ID/name. Its explicit stable edge seams and stable
corner pins are constraints, not private runtime indices. Bind the asset to the
edited entity with an ordinary command. Snapshot 17 is the proposed compatibility
boundary; empty defaults must preserve all older snapshots and hashes. An old client
must reject the new state. Existing native UV attributes need no migration.

Layout validation checks its mesh and corner/edge/attribute references and pinned
values. A later mesh edit must explicitly update or clear the layout binding; stale
constraints must not silently rebind. Shared meshes remain immutable: authoring a
UV set for one entity leaves other instances on their original asset. Other named
attributes and all topology/element IDs remain exact. Native archives retain the
original and edited state through ordinary history and recovery.

Compute unwrap/packing with cancellable bounded jobs against a pinned source revision,
then prepare/publish existing mesh and new layout commands with the same base check,
permissions, retry identity and durability as every other operation. Do not route
a long solver through the existing ModelMesh noncancelling closure. The command
path accepts typed assets but never grants an editor a separate mutation authority.

Unwrap charts are connected orientable disks after explicit seam cuts. Corner
union classes identify chart vertices without exposing transient solver numbers.
Check boundary loops and Euler/topology conditions; reject singular charts and
inconsistent pins. Automatic solver anchors are distinct from authored pins. Explicit
pins use stored f32 UV coordinates and must survive the solve exactly. Independently
implemented pivoted Householder QR solves the LSCM system; bound dense memory and
work, and report residual/pivot diagnostics rather than claiming a universal condition
number. Reject degenerate, flipped or self-overlapping chart results. Test the final
f32 coordinates, not only intermediate f64 output.

Packing is separate. Use explicit atlas resolution, pixel padding and texel-density
policy. Preserve authored pins by default; a caller must explicitly request changing
pin positions with a chart transform. Record that transform and updated pin intent.
Keep a bounded deterministic rectangle placement profile with measured occupancy;
claim neither optimal packing nor arbitrary resolution/density success. Unpacked
charts may share UV space; packed layouts must pass bounds/padding/overlap checks.

The first workflow creates a native mesh, unwraps seams, packs charts, applies a
checker texture using the named set, renders on CPU/Metal/WebGPU, then saves/reopens
and recovers exact authoring constraints. Include analytic planar/developable/bent
charts; pin conflicts, rank, orientation, bounds, overlap and budgets; pre/mid/late
cancel, stale mesh/revision, permission, retry, immutable sharing and storage failures.
Numerical math, chart analysis, transactions, evaluation and host/browser integration
all need review and evidence before the capability ledger changes.

Review found a legacy default-UV dependency on BTreeMap name order. Adding a named
set before the old first name could silently retarget unqualified texture lookups.
Add an optional explicit default UV attribute ID to Mesh. Omit None so legacy hashes
and default behavior remain exact; UV authoring freezes the previous effective ID
before inserting a set. Preserve the ID through modeling and displacement. Explicit
named material bindings keep their existing meaning. Validate reference/type/domain.

A mesh with an explicit default requires snapshot 17 even without a UV layout. The
native mesh chunk must use a new R3DMESH1 magic for that metadata; emit the original
R3DMESH0 bytes for legacy meshes. Old decoders must reject the new magic, and new
decoders must reject contradictory magic/metadata combinations. Update only the
future-version negative test boundary as version 17 becomes defined; retain actual
old-client rejection and all preexisting fixture and render artifact identities.

Resolve multiple-set constraint ownership with a typed UV Asset containing up to
8 named Layout values keyed by their stable attribute IDs and one authored mesh
hash. Each entity binds one such immutable asset. Updating one set carries every
other set's unchanged constraints forward and validates them against the generated
mesh. Do not silently replace the active constraint metadata for another UV set.

Public UV requests carry the expected source mesh and optional source UV asset key,
plus revision and idempotency key. Compute from those retained immutable sources,
so retries regenerate identical ordinary commands even after the live binding has
advanced. Check live mesh/UV bindings on fresh publication. Converted command identity
follows the existing import convention. The result returns numeric diagnostics; the
asset preserves named seams/pins/atlas intent and native history preserves exact deltas.

Integration review found that evaluated UV tables and GLB export assumed the first
named set was the default. Explicit-default meshes now place that stable attribute
ID first in evaluated UV tables, preserving legacy ordering when metadata is absent.
OBJ continues writing the effective default and reports every other set as omitted.
Tests exercise a second name sorted before the existing default, evaluated triangles,
OBJ coordinates/losses and GLB export. No old shader or material semantics change.

The animation fixture has seven triangulated boxes. Cutting every triangulation
edge exceeded the 64-chart profile; the revised fixture cuts the actual axis-aligned
box edges and retains each coplanar diagonal, producing 42 charts. This tests the
same skin/morph interaction inside the declared profile without raising its limit.
Legacy `composed()` cannot retain an animation binding after hiding its mesh; that
separate view is tested after discarding animation, while real posed evaluation
retains the ordinary authored-to-evaluated pipeline.

Final integration acceptance passed on the integrated c9b0188 baseline. Native and
Wasm authoring hashes/reports/revisions and CPU pixels agree exactly. Existing
render/pass/receipt artifacts, fixture inputs, Cargo files and all five shaders
remain byte identical. The original failed Wasm test-configuration run and input
review corrections are retained in evidence/uv-authoring/development. This qualifies only the
bounded UV engine subgate, with the documented M10/M09 work still outstanding.
