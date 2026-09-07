# Immutable evaluated geometry chunks

The bounded storage prerequisite passed [qualification](../evidence/runtime-geometry-chunks/README.md). This is a private runtime representation
change supporting the M10 locality work. Public JSON schemas, Snapshot18, durable
IDs, revision hashes, accepted transactions and render receipts do not change.

Evaluated triangle arrays and BVH node arrays use 64-item immutable slices behind
an Arc root table. A container clone shares that table and every payload. Geometry
clones still copy the small UV-attribute ID table; triangle-owned UV arrays and
node-owned leaf vectors belong to their containing chunks. BVH build and traversal
ordering remain identical. Spatial query trees still compact leaf vectors before
publication, and report the actual chunk representation layout.

A replacement takes sorted unique runtime slots in a BTreeMap and returns a new
container with copy counts. It validates every index first, copies the root table,
clones each touched chunk once, applies the values and checks cancellation before
return. Supplied slots count as touched even if their values equal the old values;
semantic no-op detection belongs to the caller. Old roots remain immutable on success, error or cancellation. Empty updates
share the original root after admission. The caller is responsible for source
revision, geometry semantics and nested payload budgets; this internal seam is not
an agent operation or permission bypass. No method exposes mutable payload access. The intended Triangle and Node payloads
have no interior mutability; generic Rust payload types remain responsible for their
own interior-mutability semantics.

Copy counts include every root reference copied and every old element cloned in a
touched chunk, including overwritten elements. Triangle UV and node item allocations
follow their Rust Clone behavior and are measured separately by the fixture. Layout
bytes include the root Vec storage, root references and chunk element payloads;
they exclude the container itself, Arc headers, allocator overhead and nested owned
allocations. Values shared by different versions must not be summed as unique
process allocation. These observations do not establish an end-to-end speedup.

Construction remains global. It builds the same temporary flat BVH and then moves
its entries into chunks; both allocations coexist during conversion. The spatial
query build charge remains conservative: binary leaves contain two to four items
except a one-item root, so node count is at most item count. The existing 512 bytes
per source item for node storage covers builder capacity, chunk payload and root
references; the fixed allowance covers tiny-tree/Arc overhead. Bounds, correspondence
and triangulation have their existing separate charges.

The chunk replacement root table is still copied in full. Evaluator invalidation,
BVH rebuilds, scene admission and GPU host packing remain global on changed geometry.
Local refit mappings, sparse authored displacement/masks, brush selection on displaced
positions and recoverable gesture transactions are later steps. Runtime chunk slots
must never become persistent point, face or corner IDs.

The reproducible [fixture](../fixtures/runtime-geometry-chunks/README.md) checks
boundary sizes, immutable old versions, invalid indices, every cancellation point,
actual Clone counts, nested payloads and an independent analytic render. Qualification
also requires existing native/browser/Metal/WebGPU/OPFS workflows, exact old rendered
outputs, source/dependency provenance and Linux/Windows compile checks.
