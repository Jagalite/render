# Static PBR integration review

## Authored data and API boundary

Snapshot v2 adds content-addressed encoded images, typed material texture roles,
PBR bindings and lens components keyed by persistent entity ID. Default-empty
maps and absent optional fields preserve v0/v1 canonical serialization. Older
engines reject v2. `put_image`, `set_camera` and `use_camera` use the existing
transaction engine, permissions, revision checks, retained receipts and durable
CAS publication. Agent variants retain their color/emission/light allowlist;
texture, lens, camera, geometry and BRDF fields stay in the protected digest.
Camera selection is a semantic command so retries do not depend on subsequently
changed render settings. There is no privileged browser/editor mutation path.

## Evaluator, Rust kernel and GPU packing

Evaluated triangles carry optional normals and signed tangents. Normal vectors
use inverse transpose; tangents use the forward transform with orthogonalization
and determinant handedness. Missing tangents use the declared per-triangle UV
basis. Degenerate interpolated normals fall back to the geometric normal.
Back faces negate all three tangent-frame basis vectors. The integration review
caught and corrected an initial partial frame reversal, then added an analytic
reflected-instance/back-face test and real rear-view CPU/Metal evidence.

GPU packing and Rust IR were reviewed together: 11 records per triangle, 16 per
instance, 10 camera/render parameter records, separate material descriptors and
an RGBA16 mip-texel buffer. The generated shader has five storage bindings,
validated by Naga and actual Metal/WebGPU execution. Texels are packed two per
vec4 record and unpacked by bit reinterpretation; no f16 shader feature is
required. Geometry/descriptor cache identity covers packed bytes; texture data
is uploaded per render. Native and browser use the same maintained Rust IR.

GGX/Schlick/separable Smith follows the named specification profile, with an
explicit 0.05 roughness floor and a one-bounce diffuse/GGX sampling mixture.
Normals, shading hemisphere checks, single/double-sided visibility, sampler
modes, projection and clipping have matched CPU/GPU paths. The flat raster
preview rejects PBR/lens requests explicitly. CPU and GPU reject nonfinite
outputs rather than publishing invalid receipts.

## Resources, dependencies and provenance

The initial 1024-pixel image limit rejected the unmodified mirrored fixture,
whose three images are 2048². That failed native run took 18.72 seconds, max RSS
92,176,384 bytes, peak footprint 89,212,608 bytes. This was a resource admission
failure, not an accepted rendering result. Full 2K support required an explicit
12×1024² decoded-pixel budget and compact GPU texture storage. Source bytes and
reference images were not resized or replaced. A subsequent debug run passed
in 149.24 seconds while other validation was active, with max RSS 588,054,528
bytes and peak footprint 1,203,228,224 bytes; the separately recorded final run
is the primary measurement. No cross-run speedup is inferred.

Only PNG/JPEG features of pinned Rust `image` are enabled. The reviewed graph
includes Rust zune-jpeg, png, flate2, zlib-rs, miniz_oxide, moxcms and half; no C
codec backend is admitted. Architecture-specific codec/color intrinsics remain
Rust-owned and target-gated; the real wasm32 build/tests passed. Native links
are operating-system graphics/services only. Generated bindings and shader IR
are distinct from maintained sources; generating commands and hashes are in
the dependency/source inventories. Khronos fixture licenses, original URLs and
hashes are retained alongside the assets and linked from the profile guide.

## Remaining qualifications

This is the first static opaque surface slice of M07. Sparse/normalized glTF
attributes, additional UV sets, alpha coverage, extensions, transmission,
animation/skins, hair, volumes, baking and richer lighting retain their gates.
Imported cameras require rigid transforms and selection snapshots their view.
Synchronous decode/import has no mid-operation cancellation handle; publication
is atomic and render cancellation retains existing CPU/GPU boundaries. GPU job
byte budgets do not bound total CPU RSS. Encoded image identity and metadata are
validated on snapshot load; full bounded codec validation occurs at ingestion
and evaluation. External URI strings are never fetched or executed.

This review covers the integration seams and fixes above. It is not an
independent visual reference-image approval or a broad compatibility claim.
