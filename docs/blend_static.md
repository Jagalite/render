# Native Blender 2.93 static profile

`blend-293-static-v1` is an independent Rust adapter for a bounded, uncompressed
Blender 2.93 scene. The [acceptance evidence](../evidence/blend-static/README.md)
qualifies this named profile. INPUT-06 remains partial.
There is no Blender/Cycles/Python runtime, conversion worker, or automatic URI fetch.

The structural reader recognizes explicit 32/64-bit pointer identifiers and little/
big byte order, embedded SDNA and structural versions 250..399. Evaluated conversion
requires 293. Serialized pointers are exact opaque relocation keys; they are never
memory addresses, native entity IDs or inferred allocation ranges. Advisory REND/
TEST metadata is retained without treating temporary writer tokens as addresses.
Raw material-slot flags use a padded four-byte block. Missing/ambiguous references,
invalid counts/layouts/links, unsupported active dependencies and nonfinite used
values fail before transaction publication.

The selected scene must have one unfiltered, enabled render view layer, at most 64
collections and 64 objects. Collection membership determines imported objects;
object parenting determines spatial transforms. Authored translations, XYZ Euler
or normalized quaternion rotations, scales and parent inverse transforms become
native transform operations. Source XYZ is represented as the native ordered Z,Y,X
product with angles in that order. Cached matrices and derived meshes are not used.
An explicit positive meters_per_unit policy scales geometry, translations and lens
clipping; the source scene's unit setting is reported separately.

Meshes retain polygon/corner order, positions, edges (including loose edges), and
named corner UVs. Limits per mesh are 16384 vertices,32768 edges/corners and 8192
polygons. Flat planar polygons with one constant material slot are supported; native
triangulation validates geometry before publication. Smooth/custom normals, multiple
material slots, non-UV render corner attributes, shape keys and deformation are
separate profiles. Unknown inert metadata remains in the exact source container.

A material must have a constant Principled node connected directly to a material
output. Base color, metallic, roughness and emission map to the native opaque
single-scattering BRDF under explicit `allow_principled_approximation` consent.
Nondefault blending/culling/shadow modes, specular, alpha, subsurface, anisotropy,
sheen, coat, transmission or linked
inputs reject. Source distribution and renderer-specific evaluation are reported
approximations. This does not promise Eevee/Cycles image identity.

Perspective and orthographic cameras support sensor fit, aspect and clipping.
Shift, depth of field and non-rigid camera transforms reject. Field of view uses
pinned pure-Rust libm atan so persisted native/Wasm camera values agree. The selected camera
becomes the initial native render camera; native camera components remain editable.
At most one non-node point light converts, with explicit
`allow_point_light_approximation` consent, to RGB times source power divided by 4pi.
Finite source radius and source-engine shadow controls are reported losses. The
result is an initial static render setting; changing the light entity later does not
retarget that setting. A constant Background node maps to the native environment.
Caller sampling/output settings apply; source display transforms and engine settings
are not reproduced.

Active modifiers, constraints, drivers/animation, simulations, linked libraries,
edit/sculpt/pose modes, collection exclusions and unsupported geometry/node types
reject individually. Inactive PartDeflect records are allowed only when both force
field and collision selectors are zero. Preserved scripts/add-on data are inert.

## Source preservation and operations

Snapshot 16 adds an optional `source_assets` table. Empty tables are omitted and old
snapshot versions/hashes remain valid. `put_source` is an ordinary transaction
command; it requires the existing write capability, base revision, idempotency and
byte allowance. Four containers/4 MiB aggregate source bytes are the table ceiling;
each container is at most 1 MiB. The content key hashes the typed asset; the conversion
report also records the raw source SHA256 and canonical asset size. Native archives
and OPFS retain those exact bytes. Source preservation is independent of editable
mesh/material/camera semantics and never grants executable support.

CLI `import` accepts `source:{format:"blend",path,policy}` and explicit sampling/
output `settings` (or an already authored recipe). The importer supplies camera and
lighting within that recipe. The shared agent/browser method is `import_blend`
with bytes, policy, settings, base_revision and idempotency_key. All persistent
changes use the same document transaction engine.

CLI `export` accepts `content:{format:"source",asset:<content-key>}` at a pinned
revision. It writes `source/original.blend` and a report through the existing output
publication mechanism. Agent/browser `export_source` takes
`request:{revision,asset}` and returns typed metadata plus original bytes. This is
exact original-source extraction: it does not write native edits back into Blender
format. Saving the native archive preserves both the edited native scene and source.
Old engines must reject snapshot 16 before evaluating it; no silent downgrade is
supported. Request/ABI envelope versions and existing operations are unchanged.

## Fixtures and validation

See `fixtures/blend-static/README.md` for the CC0 analytic corpus, primary semantic
references and the separately pinned official 293 startup input. The real fixture
and source-containing archives are development inputs, not redistributed in this
repository. Optional fetching is development tooling only.

The candidate gate covers native/Wasm positive and negative tests; CLI and browser
import-render-save-reopen; exact source extraction before/after native edits;
permissions, cancellation, stale revisions, idempotency, resource limits and storage
failure atomicity; numerical geometry/camera/lighting facts; actual CPU/Metal/WebGPU
comparison; old-engine rejection; frozen source and prior artifact identity. Runtime
resource observations must distinguish process memory from GPU allocation limits.

## Qualified workflow

179 native and 154 Wasm tests; 137 CLI calls across seven cases; exact native/Wasm import revisions and CPU pixels; actual Metal/WebGPU and OPFS recovery. 1308 prior images/passes and 436 receipts remain byte identical, with all five shaders and existing fixtures unchanged.

See the evidence README for reproduction, source provenance, failed-development
records, measured resource limits and platform qualifications.
