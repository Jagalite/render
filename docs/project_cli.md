# General project CLI contract

This originally closed the audited host-interface gaps after M06–M08. The original CLI increment added no new
geometry, material or file-format compatibility. M09 remains a separate editor
milestone. [Validation evidence](../evidence/project-cli/README.md) records 94 native
tests and an independent 36-request CPU/Metal workflow, including five shutter
frames, native restore, cancellation, diagnostics and source integrity. Subsequent
[animated glTF import](animated_gltf.md) has [separate validation](../evidence/animated-gltf/README.md). The current [multi-bounce extension](multibounce_pbr.md) has [new acceptance evidence](../evidence/multibounce-pbr/README.md).

`render-host project <project-directory> <request.json|->` executes one versioned,
typed JSON request and exits. `-` reads stdin. Relative paths are relative to the
process working directory. Successful stdout is one JSON object; failure is a
structured error on stderr with nonzero exit status. There are no fixture calls.

This is a trusted local CLI using the process owner’s filesystem permissions.
The control JSON has a fixed 16 MiB cap. The request contains `version: 0`, optional
`limits`, and `operation` with a `method`
tag. Operations cover explicit init/restore, inspect, ordinary document apply,
bounded OBJ/GLB/explicit glTF bundles, exact-time evaluation, CPU/GPU scene rendering,
CPU/GPU shutter frames/sequences, authored imaging products and document/evaluated OBJ
exports. Import resource paths are supplied by the caller; asset metadata never
causes URI, filesystem or network fetching.

Authored mutation uses the existing document transaction and native journal APIs.
Apply uses the existing Request, including revision, retry key and byte budget.
Import uses the same transaction semantics with converted typed commands. Init and
restore create a new directory and initial durable envelope; existing paths are
never replaced. Reads require an existing recoverable project. Render/export
requests pin an authored revision; animation evaluation never changes that revision.

The CLI output directory must be new. Every completed image/product bundle is
written to a pending subdirectory, synced, then renamed to its published name.
A final manifest names only published bundles and has `complete`, `failed` or
`cancelled` status. A crash before final manifest publication is incomplete.
Cancellation or failure never advertises a complete sequence. Existing outputs
are never overwritten. Artifact names use host indices, never authored names.

Limits include aggregate input bytes, output bytes, wall time and an optional
cancellation-file path. Creating that file requests cancellation; it is never
deleted by the host. CPU evaluation/transport checks cancellation at existing core
boundaries; GPU submission uses the existing cancellation flag. Native driver
initialization and an in-flight storage commit are not interruptible. Publication
finishes atomically once admitted. Process termination during publication retains
the native journal's existing recovery contract. Output limits count encoded
artifact bytes; they do not claim a whole-process allocator bound.

Default limits: 64 MiB aggregate input, 64 MiB artifact bytes, 60 seconds.
Maximums: 128 MiB input, 512 MiB output, 600 seconds. Engine geometry, texture,
render, imaging and sequence limits remain enforced as well.

Validation must create an original scene through external CLI requests, model and
animate it, recover it in a new process, evaluate random times and render a sequence.
Tests also cover malformed requests, missing projects/resources, stale revisions,
retry identity, cancellation before and during work, partial output, output budget,
existing paths, original-document roundtrip and explicit unsupported backends.
The host is a filesystem/process adapter over the same Rust computational APIs
available to native/browser clients, with no new runtime dependencies.

## Coverage audit

| Workflow | Previous host entry | General project entry |
|---|---|---|
| Create/open/recover | Implicit empty project or demo; separate host modes | Explicit `init`, existing-project `inspect`, lossless `restore` to new project |
| All supported authoring | `api` JSON lines | `apply` wraps the unchanged document Request; delta equivalence tested against `api` |
| OBJ/GLB/glTF resources | Rust calls or agent byte arrays | `import` accepts explicit local paths and converts through existing Rust adapters |
| Exact-time evaluation | Rust evaluator | `evaluate` returns instance transforms, bounds, geometry/media counts and receipts |
| Arbitrary supported scene | Agent preview limits or fixed workflow | `render` uses authored settings with explicit CPU/GPU selection and optional clip/time |
| Shutter frame/sequence | Rust calls and fixed character workflow | `frame` / `sequence` accept existing typed core requests |
| Views/color/denoising/bakes | Branch preview or fixed imaging workflow | `products` uses authored imaging configuration on the pinned project |
| Native archive / evaluated mesh | Rust calls or demo exports | `export` produces lossless document JSON or explicitly lossy world-space OBJ |
| Cancellation/output integrity | Separate APIs and workflow checks | Publication-boundary cancellation, deadline checks and atomic artifact bundles/manifests |

`render-host project --help` prints the invocation and method list. Every call
opens/recovers the project in a new process; there is no hidden current scene.
Requests and imported file bytes share the aggregate input limit. Existing journal
recovery retains the core native storage profile and is not counted as request
input. Manifest metadata is separate from encoded artifact byte accounting.
The deadline begins after request parsing and is checked at operation boundaries;
blocking filesystem reads and native driver initialization cannot be interrupted.

## Request fields

All methods are nested under `operation`. Unknown host request fields reject.
`revision` and `base_revision` are strings returned by `inspect`. IDs are canonical
32-character hexadecimal strings. Times use `{ "numerator": 1, "denominator": 24 }`.
All output paths must be new and have existing parent directories.

| Method | Fields |
|---|---|
| `init` | `document_id` |
| `restore` | `source`: exported canonical Document JSON file |
| `inspect` | None; returns snapshot, revision, document digest and document operation registry |
| `apply` | `request`: existing document Request (`version`, `base_revision`, `idempotency_key`, `commands`, `max_added_bytes`) |
| `import` | `base_revision`, `idempotency_key`, `max_added_bytes`, typed `source`, optional `settings` |
| `evaluate` | `revision`, optional `at` (`clip`, `time`) |
| `render` | `revision`, `backend` (`cpu` or `gpu`), `output`, optional `at` |
| `frame` | `request` (`revision`, `clip`, `time`, `shutter`), `output`, optional `backend` (default `cpu`); Rust CPU or GPU shutter integration |
| `sequence` | `request` (`revision`, `clip`, `times`, `shutter`), `output`, optional `backend` (default `cpu`); Rust CPU or GPU shutter integration |
| `products` | `revision`, `output`; uses authored views/bakes/color/denoising |
| `export` | `revision`, `output`, typed `content` |

Import `source` variants:

- `{ "format": "obj", "path": "mesh.obj", "entity": "<ID>", "name": "Mesh", "material": "<existing material ID>" }`. Material may be null; rendering still requires a supported material assignment. OBJ reports attribute/material losses.
- `{ "format": "glb", "path": "scene.glb", "policy": { "allow_approximations": true } }`. Uses the opaque PBR scene profile, including [supported clips, skins and morphs](animated_gltf.md). The import report maps source clip indices to native clip IDs for subsequent `at` and sequence requests.
- `{ "format": "gltf", "path": "scene.gltf", "buffers": ["scene.bin"], "images": ["albedo.png"], "policy": { "allow_approximations": true } }`. Caller-provided arrays correspond to source resource indices; no URI resolution occurs.

Imported commands append transactionally to the current project. Stable-ID
collisions reject; this interface does not silently replace the scene or invent a
new namespace. An import retry rereads its explicit resources and uses the converted command
payload as transaction identity. Changes that alter those commands reject under
the same retry key; semantically ignored metadata does not create a new mutation. Native document replacement is explicit `restore` into a fresh project.

Export `content` is `{ "format": "document" }` or
`{ "format": "obj", "entity": "<ID>", "at": { "clip": "<ID>", "time": { "numerator": 1, "denominator": 2 } } }`.
OBJ `at` is optional. The OBJ path realizes visible evaluated surface triangles in
world coordinates, including procedural geometry and deformation. It duplicates
triangle vertices and reports loss of authored topology, materials, instances,
clips and non-surface data. Native Document export is the lossless authoring path.

Shutters specify relative `open`, `close` times and `samples`; instant exposure is
zero open/close with one sample. Existing rational-time and sample-count rules
apply. Per-frame metadata retains the actual shutter times and evaluation receipt.
GPU rendering at a single clip/time is supported for compatible scene profiles.
Opaque PBR and diffuse CPU/GPU transport now supports depth 1–16 under the
[multi-bounce contract](multibounce_pbr.md). The CLI workflow compares the same
authored depth-two scene on CPU/Metal;
`frame` and `sequence` now accept optional `backend: "gpu"` under the
[GPU shutter contract](gpu_shutter.md); omitted backend remains `cpu`.

## Example

```sh
mkdir -p artifacts
printf '%s\n' '{"version":0,"operation":{"method":"init","document_id":"0000000000000000000000000000007a01"}}' \
  | target/debug/render-host project artifacts/my-project -
printf '%s\n' '{"version":0,"operation":{"method":"inspect"}}' \
  | target/debug/render-host project artifacts/my-project -
```

Use the returned revision in an `apply` request. Ordinary command arrays in
`fixtures/project-cli/create.json` and `animate.json` are example inputs; the
independent client shows how to supply current revisions and dynamic mesh IDs:

```sh
cargo build -p render-host --locked --offline
python3 scripts/project-cli-workflow.py artifacts/my-cli-run --gpu
```

That client records every concrete request and response, performs a modeled and
animated scene workflow, reopens the project through a native archive, and checks
that rendering never mutates it. It calls only the general project entry point.
