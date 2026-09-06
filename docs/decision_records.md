# Architecture decision register

M06–M08 now have [named implementation profiles](m07_m08.md) and [complete milestone evidence](../evidence/m06-m08/README.md). Historical M05 Lambertian and [static PBR](static_pbr.md) evidence retain their original scope. External input tracks remain separately qualified in `planning/input_support.json`.

The initial proposals are preserved below. M00–M04 decisions, supporting measurements, accepted limitations and revisit triggers are recorded in [accepted foundation ADRs](../evidence/m00-m04/accepted_ADRs.md). M05 integration decisions are in [accepted agent alpha ADRs](../evidence/m05/accepted_ADRs.md). Decisions outside these demonstrated profiles remain proposed. See the full specification for consequences.

## ADR-001 — Fully Rust production implementation

**Decision:** Use Rust-owned computational implementations; generated platform artifacts and narrow environmental exceptions only.

**Rejected baseline:** Rust wrappers, Cycles reuse and Blender workers.

**Revisit trigger:** Native-link audit reveals any non-permitted computational dependency.

## ADR-002 — Typed authoring database, not ECS as durable schema

**Decision:** Persist typed components/assets with stable external identity; use private optimized tables.

**Rejected baseline:** Generic ECS world or untyped property graph as the whole project format.

**Revisit trigger:** M00 shows an alternative preserves semantics while materially improving measured locality/memory.

## ADR-003 — Distinct mesh and edit representations

**Decision:** Canonical polygon/corner arrays plus handle-based radial editing topology.

**Rejected baseline:** Triangle-only assets and manifold-only half-edge kernel.

**Revisit trigger:** Conversion/memory costs break interaction budgets; change private chunking without losing topology semantics.

## ADR-004 — Scoped persistent identity

**Decision:** Global entity IDs, geometry-local element IDs and separate generational runtime handles.

**Rejected baseline:** Name/path identity, pointer identity or global UUID per transient triangle.

**Revisit trigger:** Measured ID overhead exceeds budget or edit correspondence requirements change.

## ADR-005 — Snapshots plus deltas and provenance

**Decision:** Recovery loads valid checkpoints and committed resolved deltas.

**Rejected baseline:** Re-executing an unbounded historical command log.

**Revisit trigger:** Crash/fault tests expose ambiguous durable state; revise publication protocol before format stabilization.

## ADR-006 — Multiple specialized execution graphs

**Decision:** Share schema/editor conventions while retaining shader, geometry, time and solver semantics.

**Rejected baseline:** One untyped universal graph interpreter.

**Revisit trigger:** A real shared kernel can reduce duplication without erasing domain contracts.

## ADR-007 — CPU reference plus portable GPU kernels

**Decision:** Rust CPU implementation and Rust-owned kernel IR; software traversal is the portable baseline.

**Rejected baseline:** Mandatory native hardware ray features or hand-maintained non-Rust runtime kernels.

**Revisit trigger:** M00 kernel generation fails correctness/usability/performance requirements.

## ADR-008 — Small C-compatible ABI implemented in Rust

**Decision:** Opaque handles, version negotiation and explicit buffer ownership.

**Rejected baseline:** Stable Rust trait-object/layout ABI.

**Revisit trigger:** Independent client conformance shows a missing lifecycle/error capability before ABI v1.

## ADR-009 — Branch/revision collaboration first

**Decision:** Conservative conflicts; reviewed commutativity and explicit reconciliation.

**Rejected baseline:** Automatic generic CRDT merges of arbitrary topology.

**Revisit trigger:** A precisely scoped merge can be proved safe with complete dependency information.

## ADR-010 — Native format adapters with loss reports

**Decision:** Own format-independent authoring semantics and implement scoped adapters in Rust.

**Rejected baseline:** USD/.blend runtime core or external conversion workers.

**Revisit trigger:** A documented user workflow justifies a new format/version feature profile.

## ADR-011 — Precision is a typed policy

**Decision:** f64 transforms/time computation; local f32 geometry with explicit f64 asset class and camera-relative GPU forms.

**Rejected baseline:** f32 everywhere, f64 everywhere or forced lossy matrix decomposition.

**Revisit trigger:** Scale/pathology tests demonstrate an inadequate precision class or memory trade-off.

## ADR-012 — Human editor as client

**Decision:** Ephemeral gesture previews and validated durable operations share the public contract.

**Rejected baseline:** Editor-only mutable state paths.

**Revisit trigger:** A measured interaction bottleneck requires a faster transport, not bypassing semantics.

## ADR-013 — Typed breadth and exact-time derived evaluation

**Decision:** M06–M08 add typed snapshot fields and ordinary transactions;
procedural outputs, sweeps, displaced meshes and animated poses remain derived.
Groom/skin/morph attachments use topology digests and stable local IDs. Rig binding
requires explicit triangle realization. Visibility does not remove dependency
anchors. Time and shutter arithmetic use checked rationals. GPU support is granted
per named material/geometry profile; CPU-only media, extended scattering and
imaging return explicit unsupported errors on incompatible backends.

**Integration contract:** [M06 operators](m06_modeling_contract.md),
[M07/M08 semantics](m07_m08.md) and [execution gates](m07_m08_execution.md).
Historical Lambertian and static PBR evidence remains pinned separately. Numerical
references, native workflows, browser parity, resources and source/dependency
manifests gate the capability status update.

**Rejected baseline:** Baking authored hair/rig/graph state into anonymous triangles,
using platform-private mutation paths, or silently falling back from requested GPU
profiles. Bounded named approximations remain visible in receipts.

**Revisit trigger:** New authoring or backend workflows pass their own numerical,
resource, portability and recovery gates; existing profile names keep their meaning.


## ADR-014 — General native CLI as a filesystem adapter

**Decision:** `project-cli-v0` uses a typed one-shot JSON request and ordinary
revision-checked document transactions. Host-only paths resolve explicit user
resources; imported metadata cannot trigger fetching. Rendering/evaluation calls
the existing Rust APIs, with pinned authored revisions and no fixture-generated
scene. Source/archive creation uses a new native journal directory.

Artifact bundles publish through synced pending directories and a final status
manifest. Cancellation is checked after candidate preparation at the journal
publication boundary; an admitted commit finishes atomically. Incomplete or
cancelled sequences retain only explicitly published frame bundles. Output names
use host indices so authored labels cannot become filesystem paths.

**Evidence:** [CLI contract](project_cli.md), [independent CLI/native API equivalence,
recovery, faults and resource tests](../evidence/project-cli/README.md). At that CLI validation revision, shared Rust,
Wasm and dependency sources were unchanged from the M06–M08 validated base.

**Revisit trigger:** Interactive editor/remote transports require additional
lifecycle semantics, while retaining the same public mutation authority and
backend profiles. Native driver initialization and admitted publication remain
explicit non-interruptible boundaries; Windows durability remains compile-only.

## ADR-015 — Typed absolute animation and skin-specific bind palettes

**Decision:** glTF import produces ordinary native animation components and a
collision-rejecting `merge_animation` transaction. Snapshot v11 introduces complete
absolute TRS channel groups and explicit per-skin inverse-bind palettes; existing
pose deltas and strict native rig rest/inverse-bind validation retain their meaning.
Default morph weights are authored data. Native and browser share evaluation.

**Reason:** glTF channels replace local properties, and two skins can bind the same
joint hierarchy differently. Reinterpreting native delta tracks or deriving all
skin matrices from the skeleton rest pose would change imported motion. Imported
source indices are converted to stable scoped IDs. New fields are typed, additive
and omitted when empty; old snapshot versions reject the new features.

**Integration review:** [profile and migration contract](animated_gltf.md),
[acceptance and review evidence](../evidence/animated-gltf/README.md). The workflow
crosses importer, animation, deformation, document and host/browser boundaries.
Native JSON retains converted animation; source glTF export remains unsupported.

**Revisit trigger:** Sparse/morph-normal data, additional source formats, skinning
methods or GPU shutter accumulation require separate profiles and numerical,
resource and persistence evidence. M09 UI work remains independent.

## ADR-016 — Bounded shared opaque path transport

**Decision:** Extend the existing CPU loop and Rust kernel IR to shade 1–16
opaque PBR/diffuse vertices through the existing typed max_depth setting. Preserve
primary passes, indexed samples, ordinary render transactions and depth-one CPU
semantics. Use BSDF-only sampling for emissive surfaces/environment and direct
point-light evaluation. Declare finite-depth bias, secondary LOD0 and authored
occlusion on indirect throughput. No new runtime dependency or snapshot version.

**Integration boundary:** GPU packing carries depth and sample stride in private
parameters; no public Rust layout is exposed. Browser agent preview admits the
same depth range within its existing pixel/sample limits. GPU progressive receipts
retain the selected material/depth qualifications. Advanced surfaces/media and GPU
shutter accumulation remain separate profiles. See [contract](multibounce_pbr.md).

**Revisit trigger:** Measured small-emitter variance or texture aliasing motivates
continuous light sampling/MIS or propagated secondary footprints, with independent
numerical evidence before changing the named policy.

## ADR-017 — Exact-time GPU shutter accumulation and bounded consumers

**Decision:** Share the existing frame validation/identity semantics across Rust
CPU and GPU backends. Evaluate derived poses in Rust, accumulate weighted linear
color in a persistent device buffer, and preserve nominal-time passes. A 4-byte
intermediate readback fences each dispatch; one full readback completes the frame.
Dispose temporal evaluator caches between samples. Sequence consumers accept one
completed frame at a time, with browser Promise backpressure and explicit partial
counts. No authored schema version or runtime dependency changes.

**Integration boundary:** Core exact-time evaluation, generated kernel private
packing, GPU resource/cancellation lifecycle, optional CLI backend and browser
branch authority meet at the [GPU shutter contract](gpu_shutter.md). Animated
previews never qualify a static branch for commit. Existing CPU default requests
and exact frame identities retain their meaning.

**Revisit trigger:** Measured scene rebuild/fence overhead justifies a bounded cache
or asynchronous dispatch ring with equivalent cancellation, budget and publication
semantics. Driver preemption, unbounded consumer buffering and automatic CPU
fallback are not part of this profile.

## ADR-018 — Sparse source arrays convert into existing native semantics

**Decision:** Expand checked sparse glTF accessors only within explicit temporary
limits, preserve zero/interleaved base semantics, and decode normalized unsigned
TEXCOORD_0 values into the existing UV attribute. Keep normalization role-specific.
The [accessor contract](gltf_accessors.md) adds a capability without changing typed
native geometry, animation, texture, transaction or public layout semantics.

**Integration boundary:** The shared importer serves native explicit resources and
browser byte inputs. Equivalent values retain mesh content identity; different
source bytes retain different scoped entity IDs. Sparse overrides never become
runtime-index durable IDs. Render/persistence behavior uses existing Rust APIs.

**Revisit trigger:** Larger valid sources require measured streaming expansion or
additional attribute/material semantics. Broader UV, alpha, morph and quantization
profiles must pass their own renderer/numerical and state-recovery gates.

## ADR-019 — Imported alpha uses typed CPU coverage semantics

**Decision:** Convert glTF MASK/BLEND to the existing Principled opacity model;
retain an explicit GPU rejection until its material transport is implemented.
Expand the native cutoff domain to finite nonnegative values to preserve valid
source thresholds above one. No public field layout or runtime dependency changes.
The [alpha contract](gltf_alpha.md) specifies the import and approximation limits.

**Integration boundary:** Source factor/linear texture alpha, transaction validation,
snapshot-v8 persistence, CPU ray continuation and first-sample passes must agree.
The integration review found and reproduced camera clipping after discarded hits
and orthographic depth defects; analytic tests constrain their corrected behavior.
Existing opaque import/render receipts retain their previous path and wording.

**Revisit trigger:** Derivative-filtered alpha, GPU coverage or rough transmission
requires separate numerical/image and cancellation evidence. Older engines may
reject newly admitted cutoffs above one; no silent threshold conversion is allowed.

## ADR-020 — Stable per-binding UV selection across derived geometry

**Decision:** Add an optional stable corner-UV attribute ID to texture bindings,
gated by snapshot v12. Preserve omitted-field serialization and default first-UV
semantics. Admit eight glTF coordinate sets and validate required sets on assigned
meshes. Keep private dense render slots separate from authored identity.

**Integration boundary:** Import, transactions, displacement transfer, evaluation,
CPU/GPU texture footprints, normal frames, alpha and bake sampling follow the
[named UV contract](named_uv.md). Displacement uses an explicit new policy that
preserves all UV IDs; its derived receipts intentionally change. Boxing the private
Rust command material payload does not change public JSON. No runtime dependency
changes or editor mutation paths are introduced.

**Revisit trigger:** Larger coordinate tables, texture transforms, selected bake
destinations or source-format material export require separately validated profiles.
GPU alpha remains a transport gate independent of coordinate selection.

## ADR-021 — Opt-in authored directions under morphing and affine skinning

**Decision:** Add typed stable-attribute direction offsets and per-entity authored
frame bindings, gated by snapshot v13. Apply morph deltas before the weighted
linear skin transform, inverse-transpose normals, transform tangents linearly,
adjust handedness by determinant and normalize directions. Reject singular/zero
frames explicitly. Preserve the geometric-normal default and old canonical bytes.

**Integration boundary:** Source semantic/count validation, point/corner identity,
ordinary animation transactions, immutable evaluation, frame receipts and shared
CPU/GPU consumers follow the [morph-frame contract](morph_frames.md). No shader,
ABI layout or runtime dependency changes are needed. Independent scalar oracles
separate deformation accuracy from the existing binary16 GPU texture profile.

**Revisit trigger:** Morph UV/colors, other skinning modes, reconstructed smooth
frames, or larger imported offset profiles require new numerical, resource and
recovery evidence. General animated source export remains a separate gate.

## ADR-022 — Independent source conformance complements engine oracles

**Decision:** Validate indexed attribute continuity and animation-input bounds in
Rust admission. Correct original fixture metadata while preserving every binary
geometry/image/animation payload and analytical oracle. Retain source-scoped
identity changes rather than forcing old IDs onto different source bytes.

**Integration boundary:** The [source-conformance contract](gltf_source_conformance.md)
distinguishes numerical/API acceptance from format validity. A pinned official
Khronos validator runs only as a development oracle, never a computational product
dependency. Reports retain warnings and baseline failures; corrected-source images,
depth and normals must match, while source-derived IDs/receipt hashes may change.

**Revisit trigger:** Broader source adapters require independent validity checks in
addition to typed negative, resource, persistence and rendering evidence.

## ADR-023 — Geometry-owned linear RGBA attributes

**Decision:** Add typed Vec4 attributes under snapshot v14 and one point/corner
`color_rgba` semantic per mesh. Import COLOR_0 with linear values, bounded source
clamping and explicit conversion reporting. Missing colors supply white. Keep
colors on geometry so shared materials work across colored and uncolored primitives.

**Integration boundary:** The [vertex-color contract](vertex_colors.md) ties source
admission, transactions, topology transfer, immutable evaluation, CPU alpha/bakes,
and Rust-generated GPU shading to the same barycentric semantics. Emission is
independent. Displacement preserves stable IDs and corner seams with a named policy.
Legacy diffuse/raster consumers reject colors; geometry-only exports report loss.
No runtime dependency or public material-layout change is required.

**Revisit trigger:** Multiple selected color sets, color morphs, vertex paint tools,
source material export and GPU alpha require separate bounded workflow evidence.

## ADR-024 — Evaluated PBR interchange remains separate from native authoring

**Decision:** Export embedded-resource GLB from the shared pinned evaluator with
explicit loss consent, bounded output and measured f32 world-position error. Keep
local smooth frames separate from representable world TRS transforms, retain PBR
bindings/encoded assets and map stable native UV IDs into glTF sets. Reject shading
or transform semantics that the profile cannot preserve.

**Integration boundary:** The [export contract](gltf_export.md) connects native
CLI publication and browser/agent reads to the same core implementation. Posed
morph/displacement surfaces export as evaluated triangles; report their time and
source revision. Source indices remain file-local identities. Native archives
retain authored structure, while independent Khronos validation complements render
roundtrip and recovery evidence. No runtime dependency or snapshot version changes.

**Revisit trigger:** Authored animation, hierarchy, camera/light recipes, shear
factorization, additional material extensions and larger output profiles require separate contracts and resource evidence.

## ADR-025 — Bounded GPU coverage preserves frame failure semantics

**Decision:** Admit existing Principled alpha in the Rust-generated path kernel,
keep base-level f32 coverage separately from RGBA16 color, preserve inclusive
CPU-predicate MASK thresholds, and use extended counter dimensions for stochastic
BLEND. Carry bounded traversal failure through GPU accumulation to final readback.

**Integration boundary:** The [GPU alpha contract](gpu_alpha.md) connects shared
material/UV/color meaning, camera traversal, direct visibility, CPU occlusion,
static/shutter/sequence publication and CLI/browser recovery. Existing opaque
packing and arithmetic remain the default; a separate lazily compiled kernel
variant preserves the exact baseline opaque shader. No public schema or dependency changes.
Native transactions build traversal-limit fixtures beyond the glTF node budget.

**Revisit trigger:** Secondary footprint parity, richer GPU BSDFs, exact interpolated
coverage arithmetic, larger traversal profiles or participating media require
numerical, failure, resource and native/browser acceptance evidence.

## ADR-026 — Secondary PBR footprints share the explicit base-level approximation

**Decision:** Select zero UV derivatives after an actual scattering bounce in the
extended CPU path. Reuse shared PBR shading; keep camera derivatives for all primary
coverage continuation. This aligns existing Principled, coated, conductor and
dielectric transport with the opaque CPU/GPU secondary sampling contract.

**Integration boundary:** The [footprint contract](secondary_textures.md) connects
material texture consumers, BSDF depth, alpha continuation and persisted native/browser
workflows. Filter invariance on a secondary-only emitter supplies the analytic oracle;
primary controls prevent disabling texture minification globally. No shader, schema,
ABI or dependency changes. Existing rendered artifacts and receipts remain exact.

**Revisit trigger:** Propagated ray differentials, cones or other secondary filtering
require a separate quality contract and numerical/resource evidence across backends.

## ADR-027 — Existing conductor and coat semantics receive a separate GPU variant

**Decision:** Lower the existing typed conductor and single-interface coat models
into Rust kernel IR with matched BRDF, mixture PDF and counter dimensions. Compile
conductor optical constants into F0; retain the source constants in native authoring.
Use a third lazily compiled pipeline to preserve both validated earlier shaders.

**Integration boundary:** The [surface contract](gpu_surfaces.md) connects typed
material transactions, private packing, shared texture/alpha consumers, finite-depth
transport and static/shutter/sequence publication. Analytic reflectance and zero-coat
limits complement CPU comparisons and persisted native/browser workflows. No public
schema, ABI or dependency changes; dielectric and media remain separate gates.

**Revisit trigger:** Transmission, repeated inter-layer scattering, source material
extensions, propagated footprints or broader precision profiles require distinct
numerical, failure, resource and platform acceptance evidence.

## ADR-028 — Ideal dielectric GPU transport uses geometric interface orientation

**Decision:** Lower existing ideal dielectric Fresnel, Snell/TIR and radiance eta²
transport into Rust kernel IR. Preserve geometric interface normals and admit exits
regardless of the authored one-sided flag. Keep the existing air/material interface
approximation, alpha coverage, finite depth and lack of sampled point-light caustics.

**Integration boundary:** The [dielectric contract](gpu_dielectric.md) connects typed
material validation, immutable packing, mixed BSDFs, camera/alpha traversal and
static/shutter/sequence publication. A fourth lazy shader variant preserves all three
prior programs. Independent analytic target geometry and sampled optical expectations
complement CPU/GPU and recovery workflows. No public schema or dependency changes.

**Revisit trigger:** Nested media, rough transmission, absorption thickness, source
material extensions, critical-angle precision or directly sampled glass caustics need
separate physical, numerical, failure, resource and native/browser acceptance evidence.

## ADR-029 — External VOL3 grids use explicit native cell and optical policy

**Decision:** Decode scalar density and optional aligned RGB emission in shared Rust,
with explicit coordinate units, bounds, cell-constant zero background and optical
settings. Scan bounded source bytes into the existing sparse representation; retain
no zero cells and create an empty entity for an all-zero source.

**Integration boundary:** The [VOL3 contract](volume_import.md) joins binary input,
native media, stable source identities, agent/browser admission and CLI durable
transactions. Typed reports expose coordinate interpretation and resource counts.
Existing snapshot schemas and four GPU kernels remain unchanged. Source containers
are caller-retained; native archives persist decoded values. Browser dispatch is
synchronous, while core/native cancellation checks precede atomic publication.

**Revisit trigger:** Trilinear reconstruction, spectral conversion, source round-trip
history, external paging, other formats or GPU media require separate numerical,
resource, failure and native/browser workflow evidence. This is not a simulation gate.

## ADR-030 — External HAIR strands retain native curves under explicit interpretation

**Decision:** Parse bounded HAIR polylines in shared Rust with caller-selected byte
order, metric scale, thickness meaning and color/coverage interpretation. Retain
stable strand/control identities and varying radius. Group uniform per-strand
appearances into existing native curves/materials; reject unsupported variation.

**Integration boundary:** The [HAIR profile](hair_import.md) joins binary source
admission, native sweep preflight, material coverage and transactional native/browser
persistence. Derived resource limits are checked before ordinary geometry commands
publish. No snapshot, FFI shape, shader or computational-runtime dependency changes.
The profile is a polygon-surface interpretation, not physical fiber scattering.
Disposable f64 JSON byte length is explicitly a platform-local observation; actual
evaluator receipts validate it while source identities and counts remain shared.

**Revisit trigger:** Per-control color attributes, analytic curve intersections,
physical fiber scattering, root-bound animation, source-container history or larger
paging profiles require their own semantics, resource and platform acceptance gates.

## ADR-031 — Polyline RGBA remains typed authoring through surface conversion

**Decision:** Add optional linear straight RGBA f32 to stable native polyline
controls, with white defaults and snapshot15 admission. Preserve colors through
radius subdivision, polygon rings/caps and guide-child copying using the existing
point-domain color semantic. Reject colored higher-order bases until a color error
contract exists. Old uncolored authoring and evaluated outputs remain unchanged.

**Integration boundary:** The [control color profile](curve_colors.md) spans curve,
groom, snapshot and HAIR source admission. Explicit HAIR linear_rgba_f32 policy opts
into point variation with reported alpha rounding and ordinary opaque or BLEND white
materials. Absent policy retains the original profile. Existing surface shaders and
GLB COLOR_0 export consume the derived attribute; no foreign runtime is introduced.

**Revisit trigger:** Higher-order color approximation, point colors, animated source
grooms, physical fibers or analytic intersections require separate numerical and
native/browser acceptance gates. Native authoring remains distinct from evaluated
GLB losses and platform-local disposable geometry serialization.

## ADR-032 — Bounded GPU media retains typed sparse semantics

**Decision:** Expose immutable evaluated cell inputs to a fifth Rust-IR shader for
RGB absorption/emission, with ordinary opaque PBR surfaces and zero scattering.
Preserve source cells, overlaps, metric transforms and half-open ownership. Pack
translated bounds separately from ray origins to avoid shared-face rounding holes.

**Integration boundary:** [GPU media](gpu_media.md) joins core evaluation, f32
packing, generated transport and existing CLI/browser jobs. Seven private vec4 rows
per cell have explicit work, count and corner-error bounds. No schema, FFI or Cargo
change. Existing shaders and media-free render artifacts remain byte identical.
Cancellation, progressive identity, device recreation, archives, OPFS and partial
sequence publication pass through the ordinary interfaces.

**Revisit trigger:** Scattering, alpha/advanced-surface combinations, larger sparse
acceleration/paging or stronger near-parallel precision guarantees require new
numerical, resource and native/browser evidence. CPU media semantics stay unchanged.


## ADR-033 — Static Blender conversion preserves an inert source asset

**Decision:** A bounded independent SDNA reader feeds a named Blender 2.93 static
profile. Authored scene membership, parenting, polygon/corner geometry, constant
materials and lenses become existing native commands. Explicit policy and loss
reports distinguish supported evaluation from approximated lighting/materials.
Unsupported active dependencies reject before publication.

**Integration boundary:** Snapshot 16 adds a typed optional source-asset table and
ordinary `put_source` operation. Exact source export is revision-pinned and separate
from editing or regenerating Blender files. Empty tables are omitted, preserving
older snapshots and hashes. The shared agent and CLI retain existing authorization,
idempotency, cancellation, budgets and storage publication rules. No private SDNA
layout, source pointer or executable metadata crosses into the public identity model.

**Numerical boundary:** A measured native/Wasm `atan` difference changed persisted
camera fields. Only this new field-of-view conversion uses pinned pure Rust libm;
legacy transform, transport and shader math remain unchanged. The existing package
gets one explicit core dependency edge, with provenance and resolved-feature review.

**Acceptance boundary:** See the [profile](blend_static.md) and its final validation
record. The adapter is a selected static profile, not arbitrary Blender evaluation.
Further versions, animation, node graphs, modifiers, textures, smoothing, area lights,
compressed containers or writing edited Blender files require their own fixtures,
resource bounds, compatibility tests and native/browser workflows.

## ADR-034 — UV authoring prepares ordinary mesh and constraint transactions

**Decision:** Store coordinates in typed corner attributes and retain
named seam/pin/atlas intent in immutable mesh-bound UV assets. Prepare bounded
cancellable unwrap/packing outside document command execution, then publish through
ordinary permission/revision/idempotency/storage checks. Snapshot 17 and R3DMESH1
version the explicit default UV reference and authoring metadata; legacy absent
fields remain omitted.

**Integration review:** Adding a new attribute must preserve the previous default
UV selection, other named sets and shared instances. Topology changes clear or
replace stale constraint bindings explicitly. Disposable animated meshes retain UV
coordinates while the authored constraint binding stays on their source. Public
IDs never expose chart solver indices. Packing pin movement is an explicit policy.

**Acceptance boundary:** The [qualified profile](uv_authoring.md) is an M10 engine
subgate. Numerical, transaction, archive, CLI and packaged browser tests passed,
with evidence in `evidence/uv-authoring`. Dense QR and rectangle placement have small fixed limits;
there is no general chart segmentation, optimal packing or interactive authoring
claim. Painting, sculpting and retopology remain independent requirements.
