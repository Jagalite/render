# Source preservation interface review (before implementation)

Base 9f7b4bd. The source container is an immutable provenance asset, separate from
converted entities, polygon meshes, materials, cameras and render settings. Add an
optional `source_assets` table to snapshot 16 and an ordinary `put_source` command.
Empty tables are omitted: prior snapshots keep their canonical hashes and version.
Older engines reject version 16 before evaluation; request envelope/ABI versions and
existing command spellings remain unchanged. No private SDNA index/layout is public.

Use a typed tagged format `blend` with numeric file version and exact `bytes` array,
matching current binary input conventions. A source asset validates its bounded
container (1 MiB, uncompressed structural version 250..399); preserving a container is
not evaluated scene support. The semantic importer separately requires version 293.
The table key hashes canonical typed asset bytes; reports also carry the raw source
SHA256. Limit four assets and 4 MiB aggregate source bytes. Measure canonical/document/
journal amplification before qualifying native/browser workflows. Unknown source
payloads remain inert data, including scripts, addresses and custom properties.

Only ordinary document preparation/publication can add assets. Existing principal,
base revision, idempotency, byte-admission and storage durability rules apply. Failed
conversion produces no commands at the host boundary. No source mutation, decoder
execution, remote IO or privileged editor path is introduced. Exact source export
is distinct from exporting edited entities back to a `.blend`; the latter is not
implemented by this profile. Source hashes remain protected in agent variants.

The static profile must explicitly select conversion policy for material BRDF and
source lighting. It may report deliberate approximations, but unsupported active
geometry/animation/node dependencies must reject. Source preservation is required
regardless of the selected evaluated policy. Final review still requires shared
native/Wasm workflow evidence, old-version rejection and unchanged prior artifacts.

Review findings before final acceptance: singleton/typed-array cardinality must be
checked, raw pointer arrays must have raw DATA metadata, and loop edges must join
both adjacent corner vertices. Distinct material tokens with one name must reject
before they can alias a native material ID. Selected view layers must be enabled;
object edit/sculpt/pose modes are outside the static profile. Negative source render
dimensions/pixel aspect and negative zero-power light/background colors must not be
hidden by derived ratios/products. Nondefault material blending/culling/shadow modes
need explicit rejection. These findings require source refinements and fresh final
validation; development preflights do not qualify later source changes.

The first actual browser preflight failed identical revision checks. Snapshot diff
isolated a one-bit difference in platform `atan` for both the analytic quad and real
startup camera field of view; all other persisted fields matched. Use pinned pure
Rust libm 0.2.16 atan for this new conversion only. This crate already exists in the
lockfile through shader dependencies; adding a direct core edge uses no default
features and does not alter legacy math paths or shader construction. Inspected
package/build script and atan implementation are Rust, with no native compiler/link
step; preserve package and function-level provenance and audit resolved features.
Do not round the result or weaken native/Wasm revision equality to pass this gate.

Resource review found that repeated mesh instances could duplicate full PutMesh
payloads during conversion despite final asset deduplication. Convert each source
mesh once and emit one PutMesh/PutMaterial per shared source resource. A distinct
source token with an ambiguous material name still rejects. Tests verify one asset
command/build for two spatial instances; the workflow includes a reproducible
nonoverlapping two-instance fixture. No speed claim is made from this change.

Final review also found that negative material emission RGB could disappear when
multiplied by zero strength. Reject the source RGB before multiplication, matching
the light/background admission rule; extend the native/Wasm negative-input test.
The interrupted acceptance run remains recorded and does not qualify this fix.

Final integration review accepted after the complete native/browser gate. 179 native and 154 Wasm tests; 137 CLI calls across seven cases; exact native/Wasm import revisions and CPU pixels; actual Metal/WebGPU and OPFS recovery. 1308 prior images/passes and 436 receipts remain byte identical, with all five shaders and existing fixtures unchanged.
The direct libm edge adds no package, changes no resolved native/browser feature
set and introduces no foreign computation. Exact source archives remain typed and
transactional; older clients reject both the new journal command and snapshot 16.
