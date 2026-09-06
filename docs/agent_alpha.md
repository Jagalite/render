# Agent-rendering alpha (M05)

The additive [static PBR slice](static_pbr.md) now has [separate evidence](../evidence/static-pbr/README.md): opaque textures/materials, normals and authored cameras on CPU, Metal and browser WebGPU. M07 and INPUT-01/02 remain partial; the M05 Lambertian profile and historical evidence retain their original scope.

The local workflow imports a real glTF/GLB scene, creates three restricted
variants, renders and inspects their passes, and commits one through the same
transaction engine used by other clients. The reproducible asset is the licensed
[Khronos Box](../fixtures/khronos-box/README.md). Evidence and limitations are in
[evidence/m05](../evidence/m05/README.md). This is a bounded rendering alpha.

## Run the workflow

```sh
cargo build -p render-host -p render-ffi --locked
# Output directory must be new. Camera is chosen for the Box acceptance fixture.
target/debug/render-host agent-workflow fixtures/khronos-box/Box.glb artifacts/my-m05-run --gpu
# General newline-JSON agent client, with durable root transactions:
target/debug/render-host agent artifacts/my-project
```

`agent-workflow` is a Rust CLI client of the public APIs. It saves CPU/Metal
images, receipts and depth/normal/object-ID passes; submits the same pinned
variants to durable native jobs; selects the largest mean green channel as a
reproducible test objective; commits; reopens; and retries the accepted selection.
The selection rule is a test objective, not an aesthetic assessment.
The general API accepts caller-supplied camera and lighting settings. It does not
fit arbitrary assets to the fixture camera automatically.

## Public operations

Control messages use `{ "version": 0, "operation": { "method": "inspect" } }`.
The reviewed wire shape is [agent_request.schema.json](../schemas/agent_request.schema.json).
Rust `agent::Request` and its public operation DTOs implement these semantics;
private session/branch layouts are not an ABI or a serialized workspace format.
The existing operation registry now includes the `agent.*` names below.

| Method | Meaning |
|---|---|
| `inspect` | Root revision, authored snapshot, protected digest and owned branch names |
| `import_glb`, `import_scene` | Atomic import into an empty root; explicit conversion policy and authored render settings |
| `branch` | Create an ephemeral branch at the exact root revision; repeated name/base is idempotent |
| `inspect_branch` | Branch revision, snapshot, protected digest and last successful render receipt |
| `modify` | Transaction changing only existing material color/emission and point/environment lighting |
| `preview` | Bounded synchronous CPU render with linear color, depth, world normals, object IDs and receipt |
| `prepare_commit` | Return the resolved root transaction after this exact variant has rendered |
| `commit` | Apply that selection transaction after rechecking the original root revision |
| `discard` | Release an owned transient branch |
| `export` | Export the root document; transient variants are excluded |

`modify` cannot change geometry, names, hierarchy, visibility layers, material
assignments, textures, roughness/metallic, camera, resolution, sample count or seed.
Its typed input rejects unknown properties. The protected digest also checks
those invariants before rendering and selection. Principals come from the host,
never imported text or a caller-supplied identity field. These are trusted local
clients; no remote authentication or tenant isolation is claimed.

Imports and edits require a base revision and a 16–128-byte idempotency key.
Native `agent` and `api` use the same local principal. Retain the transaction
returned by `prepare_commit`: after losing a commit acknowledgment or restarting
the agent process, send that transaction to `render-host api <same-project>` to
recover the durable receipt. In-process `commit` retries also return the retained
receipt. Changed payloads with the same key fail. Stale bases fail conservatively.
A failed native publication leaves in-memory root and branches unchanged.

Branch workspaces and preview receipts are transient. A reload restores the
selected root and retained transaction receipts; it does not restore unfinished
variants. Export is an explicit backup, separate from origin-managed storage.

## Static scene profile

`gltf2-static-lambertian-v0` supports glTF 2.0 JSON with explicitly supplied
buffers, or GLB 2.0 with JSON then one embedded BIN chunk. It resolves no URIs and
executes no metadata. The selected scene preserves node hierarchy, affine
matrix/TRS transforms, shared geometry instances, multiple triangle primitives,
material assignments, unsigned 8/16/32-bit indices, nonindexed triangles,
float32 positions/normals and a corner UV set. Source node/material IDs are
persisted identities derived from the document namespace and source digest;
the import report records the source-index mapping.

Material conversion requires `allow_lambertian: true`, metallic factor zero,
roughness one, opaque alpha and no textures/extensions. The report explicitly
states that dielectric specular is omitted, rendering is two-sided and normals
are geometric. Imported normals are retained as typed attributes, not evaluated
as a smooth-shading model. No PBR compatibility claim follows.

Cameras in the input file, animations, skins, morphs, textures/images, additional
vertex semantics, nontriangle modes and extensions
are rejected. A caller supplies the camera separately. Extras/generator metadata
and unselected scenes are identified as losses. The old mesh-only OBJ/glTF APIs
remain separate. INPUT-01 is partially implemented; its complete static scene,
texture and PBR requirements remain in the [input plan](input_support.md).

Admission limits: 4 MiB combined source, 64 nodes/meshes/materials/primitives,
20,000 total decoded points plus corners, and 256 commands per import transaction.
The final evaluated import must fit the agent profile: 192 entities and an 8 MiB
canonical snapshot. There are three live variants per session. CPU/GPU previews
are limited to 128×128 pixels, 64 samples and one diffuse bounce. Output admission
charges 64 bytes/pixel; it is not an assertion about total process RSS. Source,
transaction, branch and renderer allocations have separate bounds. Measured
process resources are reported separately.

## Native jobs and recovery

`render-host jobs <project>` retains the existing asynchronous submit/status/
cancel/events API and adds `submit_input` with `{id, input:{snapshot,settings},
budget}`. The host validates and durably pins the supplied branch. It never
publishes that branch as the root. Repeating an ID with the same owner/input/
budget returns the prior job; different content is an error. Native jobs retain
at most 256 records per project, 64 queued and eight active per principal; event
retention is 256. Archiving is manual; automatic deletion is not part of M05.

Successful jobs publish PPM, render receipt and structured-pass artifacts before
recording success. Output-budget failures publish no usable result. Cancellation
and terminal events persist. Event cursors resume after reconnect; expired
cursors require status resynchronization. A running job interrupted by process
termination is recovered as failed, not silently rerun. Native persistence uses
the existing fsync/rename/directory-sync journal with an exclusive worker lease.

## Browser client

```sh
sh scripts/build-web.sh
target/debug/render-host serve web 8765
```

Generated bindings expose `BrowserAgent.new(id)`, `dispatch(json)`,
`preview_gpu(branch, revision)`, `cancel_preview()`, `export_json()`,
`BrowserAgent.import_json(json)`, `save(name, expected_document_digest, abort)` and
`BrowserAgent.load(name)`. The page remains a Rust conformance application; the
agent is a general binding API, not a new visual editor.

GPU admission/cancellation registration occurs before the returned Promise.
Only one GPU preview runs per session. Cancellation discards the result; it does
not promise immediate GPU preemption. Completion rechecks the pinned revision.
CPU `preview` is synchronous; use the async GPU binding for interactive browser
cancellation. Native durable jobs have their own worker, while browser jobs do
not survive tab closure. This limitation is explicit.

A named OPFS save uses a Web Lock and compares the full stored document digest
before close. `inspect` and save acknowledgments expose `document_digest`. Create
uses an absent expected digest; replacement requires the exact stored digest.
This also prevents losing retained receipts when two transactions leave the
authored revision unchanged. Quota failure, abort and unclosed replacement preserve the prior
committed project. Saving returns a storage-class acknowledgment, not a native
fsync guarantee. A browser may evict origin storage; keep exported backups.

Documents with authored render settings use snapshot version 1. Version 0
snapshots without settings retain their original hashes and remain readable.
Older implementations reject version 1 rather than dropping its lighting.
The journal envelope remains version 0; operation versions are independent.

## Narrow ABI v1

`render_entry(1, sizeof(RenderApi))` negotiates the table in
[include/render.h](../include/render.h). Version 0 remains available. Unknown
versions or a requested minimum larger than the table return null. A minimum
prefix size may be requested; callers must check `struct_size` before reading
members beyond that prefix. The table lives for the process lifetime.

The six functions are create, destroy, request, response_size, response_read and
release. All scalars have fixed widths, IDs are passed as high/low u64 halves,
and handles encode slot generations. Zero signals creation/request failure;
destroy/release return zero on success and -1 for invalid type/generation.
`response_read` returns -1 for an invalid buffer and -2 for null/insufficient
output. A zero response size means invalid handle; valid responses are nonempty.

Requests contain UTF-8 JSON, at most 16 MiB. Existing `inspect`, `execute` and
`registry` envelopes remain available. Agent calls use
`{"method":"agent","request":<agent request>}`. Responses are `{"Ok":...}` or
`{"Err":{"code","stage","message","retryable","context"}}`. Binary table
version 1 does not freeze operation version 0 or private document serialization.

Returned buffers own immutable bytes independently of engine lifetime. Clients
copy into their own storage and release each buffer exactly once. Wrong handle
kinds and stale generations reject. The registry retains at most 128 live
engine/buffer handles and 32 MiB of response bytes. Allocation failure can lose
an acknowledgment; retry mutations with their retained idempotency key.

Calls may originate on any host thread and serialize under a global lock.
There are no callbacks, borrowed Rust layouts, or client reentrancy while locked.
This ABI is synchronous and memory-only; asynchronous native jobs and durable
publication use the host adapters above. Callers own valid pointer/length
regions for each call; an in-process ABI is not a sandbox. Caught unwinds poison
and quarantine the registry; fatal faults require process isolation/restart.
The independent Python and C11 clients validate the table before its v1 status
is published. C and Python files in `scripts/` are conformance tools, not product
computational implementations.

## Reproduce verification

Run `python3 scripts/validate-agent.py` with the local server above and an
isolated Chromium instance using CDP port 9223. On macOS the measured browser was
launched with `--headless=new --remote-debugging-port=9223
--user-data-dir=/tmp/render-m05-chrome --enable-unsafe-webgpu --use-angle=metal`.
The script records exact commands/logs for native/WASM tests, strict Clippy,
Linux/Windows compile checks, ABI clients, native stream/retry faults, dependency
inspection, Metal workflow timing/RSS, browser workflow and foundation regressions.
It writes to `artifacts/m05` and never updates reference images automatically.

The additive [M06–M08 contracts](m07_m08.md) expose ordinary typed authoring commands, pinned animated previews, CPU extended root rendering and imaging products. These operations preserve the existing narrow ABI and restricted static-variant commit contract.

The shared importer now additionally accepts [bounded sparse accessors and
normalized unsigned UV0](gltf_accessors.md); other normalized vertex roles retain
their explicit profile restrictions.
