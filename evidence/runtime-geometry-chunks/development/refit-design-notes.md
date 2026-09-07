# Next bounded change: refit ownership notes

These are design notes, not an accepted public schema or qualified implementation.
The storage candidate must finish first.

A refit consumer needs immutable item-to-leaf and node-to-parent maps plus current
item bounds. The existing builder is preorder: children always follow their parent.
Collect affected leaves from changed items, recompute each leaf using every item in
that leaf, then recompute the unique ancestor set in descending node order. Count
item-bound reads, affected leaf/node work, metadata traversal, copied root references,
payload clones and retained mapping arrays. Cancellation never publishes a partial
state. Changed bounds must be finite and ordered; emptiness and fixed topology are
explicit, not encoded as NaNs or inverted boxes.

Avoid charging every ordinary scene/query BVH for sculpt-only maps. A scoped refit
state can retain a shared immutable basis and own read-only current bounds/tree;
construct maps once from a valid basis. Expose its current tree by shared reference,
not mutable ownership. Generic public Node fields cannot be trusted as refit lineage:
keep the refit state's basis and current tree private and construct them through its
validator, with no method accepting an arbitrary replacement tree.

Fresh reconstruction must use the SAME immutable basis partition followed by current
bounds. Building a fresh BVH from displaced positions can reorder equal-distance
triangle candidates. Refit topology never changes children or item order. Test exact
candidate order and CPU/GPU images against reconstruction from that shared basis,
including coincident triangles, negative coordinates and large displacements. Refit
quality can degrade after large moves; observe traversal cost separately from update
cost and reserve a deliberate repartition policy for later work.

Sculpt geometry then needs stable point/corner-to-triangle correspondence, immutable
base triangulation, sparse f32 position overlays, affected triangle normal updates
and conservative material/skin/morph interaction rules. A brush must query displaced
positions. Runtime chunk indices are not authored identities; mappings remain keyed
by stable domain IDs and base topology hashes. Snapshot admission/hash and GPU host
packing remain global even after a local tree refit is demonstrated.
