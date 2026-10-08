# Render: full Blender audit roadmap expansion

Prepared 7 October 2026. Documentation-only planning update.

104 concrete, open requirements from all 16 source-audited domains plus the NPR refinement are assigned to existing milestone owners. Every one has dependencies, acceptance criteria, evidence paths and separate qualification axes. No milestone is marked complete by this update.

## Baseline and goal

Blender 5.2.2: `d13f752e3b9c4f8c261cda552b1021f8bcc0382c`. Qualified Render audit baseline: `7fff667deb967728c56d539b48361323502143aa`. Local roadmap base: `97974d9323e4e15411200e5191808b997644d962`.

Long-term 100% parity against an explicitly versioned and evolving Blender application capability inventory. This audit-derived list is the complete captured gap list, not an exhaustive enumeration of every Blender behavior.

The canonical ledger is [planning/blender_parity_matrix.json](../planning/blender_parity_matrix.json); [milestones.json](../planning/milestones.json) links every owning milestone to its requirement IDs. The preserved [source-grounded audit](reference/blender-5.2.2-parity-audit.md) supplies domain-by-domain observations and pinned source references.

## How completion works

1. Freeze the precise Blender 5.2.2 semantics, defaults, supported cases and negative cases in a versioned fixture inventory before claiming equivalence; newly found behavior becomes new rows rather than disappearing into exclusions.
2. Provide independently authored positive/adversarial and numerical/property fixtures appropriate to this capability, with pinned reference scenes/settings, tolerances and resource budgets agreed before measuring.
3. Exercise public transactions, cancellation, stale revisions, undo/recovery and save/reopen; record any inapplicable requirement with a reviewed reason, never a silent pass.
4. Record each authoring/evaluation/final-render/preview/human-UI/import/edited-export axis separately per native OS/device/backend and live browser; unsupported, blocked and not-run outcomes remain visible.
5. Publish exact Render source revision, fixture hashes, commands, dependency/provenance and independent review with every result. Historical bounded evidence cannot qualify broader semantics or modified bytes.
6. Use independently written Rust designs and original/licensed fixtures; no copied or translated Blender GPL implementation and no Blender/Cycles runtime shortcut.

All requirements below start as planned follow-on scope with qualification not run. Each row has seven independent axes: authoring, evaluation, final render, preview, human UI, import and edited export. Each axis must list actual OS/device/backend/browser evidence. Pass, fail, blocked, unsupported, not run and reviewed not applicable remain distinct. Not-applicable needs a reviewed semantic reason; it must not hide a parity gap.

Required evidence per row: inventory.json, conformance.json, workflow.json, negative_recovery.json, resource_profile.json, platform_axes.json and review.md, under the exact evidence path listed below. Evidence paths are requirements, not claims that files exist. Resource tolerances and budgets are frozen before measuring; they are not invented in this plan.

A milestone owner is a responsible subsystem role, not an invented staffed person. Prerequisite milestones preserve the existing acyclic dependency graph. Separately listed cross-domain integration contracts must be sliced into versioned interfaces and fixtures before implementation; they are required for full-row qualification but are not whole-milestone start or mutual-completion gates.

M19 remains independently startable from M02/M13. The later M19-A collection/view-layer implementation and M19-B entity-selection-set/inert-metadata implementation are bounded candidates, separate from the qualified 7fff667 audit baseline and the still-open linked-assets/override requirements. M15/M16 cleanup qualifications and original M09/M10/M12/M14 release gates remain open.

Full bpy compatibility is an explicit program and architecture/scope decision, separate from native extensions and migration tooling. The Rust-only/runtime/security constraints stay binding; roadmap inclusion does not authorize Python/Blender embedding. Arbitrary third-party add-ons and historical file behavior require a published extension/legacy policy. Authenticated collaboration remains a Render product goal rather than an assumed Blender feature deficit.

Existing holds remain active: M16/C2 resource cases, five receipt/review tests and standalone/equivalent receipt probes, browser runtime and remote publication. This planning change neither executes nor authorizes those actions.

## Post-audit implementation progress

The [M19 progress record](m19_implementation_progress.md), version
`m19-post-audit-progress.11`, records twenty-six bounded increments through
`722b7f80437e14014a74c957b5763ad1b16e496b`. Native member transform/material Set/Clear
and fresh material copying now support per-placement art direction with owner-only
selection. Transforms store full value replacements; material maps reference
destination Material IDs resolved from the current table. Fresh copies clone full
values at creation but can be edited later through applicable public operations;
old copies remain retained. No sparse live-source fields are implied. Explicit ClearAll Relink adds separate all-placement/loss review,
while default RequireEmpty and preserve-only Refresh remain.

Thirteen new headless tests pass with sixteen native-library and nine owner-preview
regressions. Tiny public workflows and valid texture checks do not qualify physical
OS/window/browser/GPU, maximum resources, security or independent-renderer fidelity.
Selective native correspondence, geometry/source hierarchy editing and general/
nested/live libraries remain product gaps; they are not automatically extra literal
M19 blockers. Core/Host/CLI already support bounded migration. Named reactivatable
variants support a bounded preset interpretation; broad catalogs stay under M45.
The [read-only acceptance crosswalk](m19_acceptance_checkpoint.md) is consolidated
without new qualification: resource coverage is partial,
and owner-mode view/variant selection remains a UI gap despite core/CLI support.
No next implementation or execution is promised.

Full M19 remains partial. All original 47 milestone fields/statuses, 104 full-row
axes, dependencies, prior evidence and held paths remain unchanged.

## M27 fixed-size color-math prerequisite progress

The separate [M27 prerequisite record](m27_color_math_progress.md), version
`m27-color-math-prerequisite-progress.1`, pins profile
`rec2020-d65-st2084-absolute-v1` to source
`3e3abcdc69e4ebd3b915e6a8204190356de24fba`. It records reviewed fixed-size native
math evidence for Rec.2020/D65 coordinates, explicit reference-white mapping and
absolute PQ EOTF/inverse EOTF. No HDR image/display workflow or full-row axis is
qualified. M27 remains planned with unfinished M23; it is not dependency-ready.
The frozen M19 progress, history and acceptance crosswalk are unchanged.

## M28 planar-chain IK prerequisite progress

The separate [M28 prerequisite record](m28_planar_chain_ik_progress.md), version
`m28-planar-chain-ik-prerequisite-progress.1`, pins `PlanarChainIkV1` to
`57d6b02bee76a56f3a17ebace354cc43b8512dc8`. Nineteen native geometry-free tests and a public
example cover bounded planar 3–16-link direct pose evaluation, public authoring
and authored-Snapshot JSON reload. The snapshot-40 profile is explicitly rejected
by aggregate animation before composition. No deformation, render/export, editor,
journal/crash recovery, resource/security or platform qualification is claimed.
M28 remains planned with unfinished M18; all full-row axes, historical M27 records
and frozen M19 progress remain unchanged.

## Full acceptance list

### Modeling, topology, modifiers — Q/P; general bevel/Boolean A

#### BP522.modeling.selection_region — General selection and region continuity

Owner: M15 · Geometry + topology + verification
Prerequisite milestones: M02, M06
Cross-domain integration contracts: M16
Status: planned_follow_on_scope; qualification not_run

Author mixed point/edge/face regions on loose, non-manifold and admissible nonplanar inputs; preserve selection identity and edit continuity across topology changes, undo and reload.

Evidence: `evidence/parity/blender-5.2.2/modeling/selection_region/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.cut_fill_repair — Cutting, filling and topology repair

Owner: M16 · Geometry + numerics
Prerequisite milestones: M15
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Cut intersecting regions, fill general admitted holes and repair explicitly selected defects while recording attribute correspondence and loss; reject ambiguous repairs and preserve untouched components.

Evidence: `evidence/parity/blender-5.2.2/modeling/cut_fill_repair/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.cleanup_completion — Remaining general cleanup subgates

Owner: M16 · Geometry + numerics
Prerequisite milestones: M15
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Complete remaining C2–C4 and broader cleanup contracts with explicit survivor/loss policy, durable baked-loss provenance and independent qualification; existing C1b–C1e bounded evidence stays frozen and held work stays held.

Evidence: `evidence/parity/blender-5.2.2/modeling/cleanup_completion/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.bevel_general — General attributed bevel

Owner: M17 · Robust geometry + topology review
Prerequisite milestones: M16
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Bevel variable-width non-box mesh edges and junctions with material/UV/normal transfer, overlap policy and manifold/non-manifold diagnostics; box bevel evidence cannot satisfy this row.

Evidence: `evidence/parity/blender-5.2.2/modeling/bevel_general/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.boolean_general — General mesh Boolean operations

Owner: M17 · Robust geometry + topology review
Prerequisite milestones: M16
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Union, intersect and subtract nonconvex attributed meshes with coplanar/tangent/degenerate adversarial cases, stable correspondence and explicit exactness/tolerance policy; BooleanBox does not count.

Evidence: `evidence/parity/blender-5.2.2/modeling/boolean_general/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.subdivision — Reusable iterative subdivision

Owner: M18 · Geometry + animation + numerics
Prerequisite milestones: M15, M16
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Evaluate repeated subdivision with boundaries, creases, UV seams and arbitrary admitted polygon valence, preserving edit/evaluated distinction and numerical convergence.

Evidence: `evidence/parity/blender-5.2.2/modeling/subdivision/` (all seven required reports; shared acceptance profile applies).

#### BP522.modeling.deform_modifier_stack — Deformation and modifier ordering

Owner: M18 · Geometry + animation + numerics
Prerequisite milestones: M15, M16
Cross-domain integration contracts: M28
Status: planned_follow_on_scope; qualification not_run

Compose deformation and topology modifiers in an ordered stack with dependency invalidation, attribute transfer, edit-cage behavior and reorder/reload equivalence.

Evidence: `evidence/parity/blender-5.2.2/modeling/deform_modifier_stack/` (all seven required reports; shared acceptance profile applies).

### Sculpt, remeshing and retopology — P/I

#### BP522.sculpt.brush_strokes — Complete brush assets and strokes

Owner: M31 · Sculpt + topology + spatial queries
Prerequisite milestones: M16, M17, M18, M30
Cross-domain integration contracts: M33
Status: planned_follow_on_scope; qualification not_run

Author reusable brush assets with pressure/spacing/falloff/textures and repeatable long strokes; cover draw/smooth/flatten plus topology-, pose- and cloth-aware behaviors and automasking.

Evidence: `evidence/parity/blender-5.2.2/sculpt/brush_strokes/` (all seven required reports; shared acceptance profile applies).

#### BP522.sculpt.voxel_remesh — Volumetric remeshing

Owner: M31 · Sculpt + topology + spatial queries
Prerequisite milestones: M16, M17, M18, M30
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Remesh closed and open inputs using a declared voxel surface policy with resolution/error bounds and explicit UV/color/mask/face-set transfer or consented loss; vertex clustering is not equivalence.

Evidence: `evidence/parity/blender-5.2.2/sculpt/voxel_remesh/` (all seven required reports; shared acceptance profile applies).

#### BP522.sculpt.dyntopo — Adaptive dynamic topology

Owner: M31 · Sculpt + topology + spatial queries
Prerequisite milestones: M16, M17, M18, M30
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Adaptively split/collapse geometry during strokes across uneven density with quality/error controls, masks and attribute/loss policy; selected-triangle splitting alone does not satisfy this row.

Evidence: `evidence/parity/blender-5.2.2/sculpt/dyntopo/` (all seven required reports; shared acceptance profile applies).

#### BP522.sculpt.multires — Editable multiresolution sculpt

Owner: M31 · Sculpt + topology + spatial queries
Prerequisite milestones: M16, M17, M18, M30
Cross-domain integration contracts: M28
Status: planned_follow_on_scope; qualification not_run

Sculpt, switch and propagate levels with displacement preservation, deformation integration and base-topology restrictions; distinguish this from triangle-centroid prolongation.

Evidence: `evidence/parity/blender-5.2.2/sculpt/multires/` (all seven required reports; shared acceptance profile applies).

#### BP522.sculpt.retopo_latency — Evaluated retopology and dense assets

Owner: M31 · Sculpt + topology + spatial queries
Prerequisite milestones: M16, M17, M18, M30
Cross-domain integration contracts: M28, M40
Status: planned_follow_on_scope; qualification not_run

Snap retopology to evaluated/deformed surfaces with occlusion/normal controls and demonstrate fixed dense-asset latency, sustained strokes, undo/recovery and resource budgets.

Evidence: `evidence/parity/blender-5.2.2/sculpt/retopo_latency/` (all seven required reports; shared acceptance profile applies).

### UV, texture/vertex/weight paint and baking — Q/P/I

#### BP522.uv_paint.uv_topology — General UV topology and unwrap

Owner: M30 · UV + paint + imaging + devices
Prerequisite milestones: M15, M18, M23, M27, M28
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Unwrap multiple chart topologies with cuts, holes, pins and seam continuity; measure distortion against fixed reference vectors and preserve named UV sets.

Evidence: `evidence/parity/blender-5.2.2/uv_paint/uv_topology/` (all seven required reports; shared acceptance profile applies).

#### BP522.uv_paint.pack_multitile — Packing and multi-tile authoring

Owner: M30 · UV + paint + imaging + devices
Prerequisite milestones: M15, M18, M23, M27, M28
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Pack nonrectangular charts with rotation, padding, scaling, pin constraints and multi-tile/UDIM semantics; preserve tile addressing through editing, sampling and interchange.

Evidence: `evidence/parity/blender-5.2.2/uv_paint/pack_multitile/` (all seven required reports; shared acceptance profile applies).

#### BP522.uv_paint.paint_brushes — Texture painting stroke breadth

Owner: M30 · UV + paint + imaging + devices
Prerequisite milestones: M15, M18, M23, M27, M28
Cross-domain integration contracts: M40
Status: planned_follow_on_scope; qualification not_run

Implement clone, smear and textured/projected brush strokes with seam/discontinuity handling, repeatable sampling, tiled targets and cancellation/recovery across long strokes.

Evidence: `evidence/parity/blender-5.2.2/uv_paint/paint_brushes/` (all seven required reports; shared acceptance profile applies).

#### BP522.uv_paint.vertex_weights — Vertex colors and weight painting

Owner: M30 · UV + paint + imaging + devices
Prerequisite milestones: M15, M18, M23, M27, M28
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Paint/interpolate normalized and locked vertex groups and named color attributes with symmetry/masking, deformed-surface selection and undo; retain values through rig edits and export.

Evidence: `evidence/parity/blender-5.2.2/uv_paint/vertex_weights/` (all seven required reports; shared acceptance profile applies).

#### BP522.uv_paint.material_bake — Material-aware baking

Owner: M30 · UV + paint + imaging + devices
Prerequisite milestones: M15, M18, M23, M27, M28
Cross-domain integration contracts: M24, M25
Status: planned_follow_on_scope; qualification not_run

Bake documented shader/pass outputs with cage/ray-distance controls, selected-to-active projection, occlusion/normal-space policies and seam dilation; verify material-aware numeric outputs and loss reporting.

Evidence: `evidence/parity/blender-5.2.2/uv_paint/material_bake/` (all seven required reports; shared acceptance profile applies).

### Curves, surfaces, text and implicit geometry — Q/P; general authoring A

#### BP522.geometry.curves — Editable curves and groom representations

Owner: M20 · Geometry families + text + graphics
Prerequisite milestones: M16, M18, M19
Cross-domain integration contracts: M25
Status: planned_follow_on_scope; qualification not_run

Edit supported spline bases, handles, knots, tilt/radius and topology with shape-preserving conversions; keep authored, evaluated, sweep and analytic render representations distinct.

Evidence: `evidence/parity/blender-5.2.2/geometry/curves/` (all seven required reports; shared acceptance profile applies).

#### BP522.geometry.surfaces_lattice — Parametric surfaces and lattices

Owner: M20 · Geometry families + text + graphics
Prerequisite milestones: M16, M18, M19
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Author/edit parametric surfaces and lattice cages with continuity/tessellation/degeneracy policies and deformation tests; preserve authored control data through save/export.

Evidence: `evidence/parity/blender-5.2.2/geometry/surfaces_lattice/` (all seven required reports; shared acceptance profile applies).

#### BP522.geometry.text — Shaped editable text

Owner: M20 · Geometry families + text + graphics
Prerequisite milestones: M16, M18, M19
Cross-domain integration contracts: M41, M45
Status: planned_follow_on_scope; qualification not_run

Shape and edit text with fonts, fallback, Unicode/layout, curve conversion and extrusion/bevel policies; preserve editable text intent and diagnose missing font dependencies.

Evidence: `evidence/parity/blender-5.2.2/geometry/text/` (all seven required reports; shared acceptance profile applies).

#### BP522.geometry.implicit — Implicit and metaball authoring

Owner: M20 · Geometry families + text + graphics
Prerequisite milestones: M16, M18, M19
Cross-domain integration contracts: M25
Status: planned_follow_on_scope; qualification not_run

Edit implicit primitives and blends with controlled iso-surface evaluation, normals and conversion error; distinguish authored field semantics from baked polygon output.

Evidence: `evidence/parity/blender-5.2.2/geometry/implicit/` (all seven required reports; shared acceptance profile applies).

### Geometry Nodes and procedural execution — Q/P; stateful breadth A

#### BP522.nodes.fields_domains — Multi-domain fields and attributes

Owner: M21 · Procedural geometry + API
Prerequisite milestones: M15, M19, M20
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Evaluate typed fields on points, edges, faces, corners, curves and instances with named/anonymous attribute propagation, domain conversion and explicit lazy evaluation semantics.

Evidence: `evidence/parity/blender-5.2.2/nodes/fields_domains/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.instances — Instance propagation and realization

Owner: M21 · Procedural geometry + API
Prerequisite milestones: M15, M19, M20
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Compose nested instances and per-instance fields without accidental realization; define transforms, identity, attribute capture and explicit realization costs/losses.

Evidence: `evidence/parity/blender-5.2.2/nodes/instances/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.ports_groups — Node inventory, groups and typed ports

Owner: M21 · Procedural geometry + API
Prerequisite milestones: M15, M19, M20
Cross-domain integration contracts: M43
Status: planned_follow_on_scope; qualification not_run

Inventory every pinned Geometry Nodes family and implement typed sockets, reusable groups, defaults/conversions, group interfaces and version migration with positive and invalid-link fixtures.

Evidence: `evidence/parity/blender-5.2.2/nodes/ports_groups/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.bundles_lists — Bundle/list data and sockets

Owner: M21 · Procedural geometry + API
Prerequisite milestones: M15, M19, M20
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement named bundle sockets and list-valued procedural data with construction, access, propagation, field/context rules, serialization and incompatible-type diagnostics.

Evidence: `evidence/parity/blender-5.2.2/nodes/bundles_lists/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.grid_sdf — Grid and signed-distance fields

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: M35
Status: planned_follow_on_scope; qualification not_run

Author/sample/convert grids and SDFs, including Boolean operations and advection, with sparse/background/topology semantics, resolution controls and numerical field fixtures.

Evidence: `evidence/parity/blender-5.2.2/nodes/grid_sdf/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.zones_time — Simulation and repeat zones

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement repeat and simulation zones with state initialization, rational time stepping, boundary semantics, nested evaluation and deterministic replay; test modifier-versus-tool context restrictions.

Evidence: `evidence/parity/blender-5.2.2/nodes/zones_time/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.cache_bake — Stateful cache and bake invalidation

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: M45
Status: planned_follow_on_scope; qualification not_run

Bake, persist, replay and invalidate state on graph/input/time changes; recover interrupted bakes, handle missing/corrupt caches and compare sequential versus random-time requests.

Evidence: `evidence/parity/blender-5.2.2/nodes/cache_bake/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.node_tools — Interactive node tools

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: M40
Status: planned_follow_on_scope; qualification not_run

Expose node tools through the public transaction/editor path with selection/context, modal inputs, cancellation, undo, invalid-context diagnostics and save/reopen equivalence.

Evidence: `evidence/parity/blender-5.2.2/nodes/node_tools/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.sound_geometry — Sound-driven geometry

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: M38
Status: planned_follow_on_scope; qualification not_run

Sample audio frequency bands at exact rational times to drive geometry; define FFT/window/channel/amplitude policies and test resampling, seek, silence and discontinuities.

Evidence: `evidence/parity/blender-5.2.2/nodes/sound_geometry/` (all seven required reports; shared acceptance profile applies).

#### BP522.nodes.xpbd_solver — XPBD solver node

Owner: M22 · Procedural evaluation + editor + simulation
Prerequisite milestones: M21, M11
Cross-domain integration contracts: M32, M33, M34, M35
Status: planned_follow_on_scope; qualification not_run

Provide node-based XPBD with translational and rotational state, constraints, collision and friction; test timestep/iteration sensitivity, determinism, cache invalidation and coupling rather than only a cloth membrane.

Evidence: `evidence/parity/blender-5.2.2/nodes/xpbd_solver/` (all seven required reports; shared acceptance profile applies).

### Materials and path transport — Q/P; full shader graphs A

#### BP522.materials.shader_graphs — Typed shader graphs and texture nodes

Owner: M23 · Shading + GPU + editor
Prerequisite milestones: M07, M19, M21
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Author and evaluate the pinned shader-family inventory with typed links/groups, defaults, procedural textures, coordinate spaces and texture authoring; validate cycles and backend capability diagnostics.

Evidence: `evidence/parity/blender-5.2.2/materials/shader_graphs/` (all seven required reports; shared acceptance profile applies).

#### BP522.materials.rough_transmission — Rough reflection and transmission

Owner: M24 · Scattering + transport + numerical review
Prerequisite milestones: M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement rough dielectric/conductor transport with anisotropy and documented parameter mapping; verify energy/PDF consistency, reciprocity and statistical reference fixtures across roughness/IOR limits.

Evidence: `evidence/parity/blender-5.2.2/materials/rough_transmission/` (all seven required reports; shared acceptance profile applies).

#### BP522.materials.subsurface — Subsurface transport

Owner: M24 · Scattering + transport + numerical review
Prerequisite milestones: M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement declared subsurface/random-walk or diffusion profiles with scale, boundary and sampling semantics; test slab/sphere profiles, energy and convergence rather than surface tint approximations.

Evidence: `evidence/parity/blender-5.2.2/materials/subsurface/` (all seven required reports; shared acceptance profile applies).

#### BP522.materials.physical_hair — Physical fiber scattering

Owner: M24 · Scattering + transport + numerical review
Prerequisite milestones: M23
Cross-domain integration contracts: M20
Status: planned_follow_on_scope; qualification not_run

Implement independently designed fiber scattering matching declared physical-hair outcomes, orientation and roughness controls; qualify fiber geometry and multiple-lobe sampling rather than swept surface shading.

Evidence: `evidence/parity/blender-5.2.2/materials/physical_hair/` (all seven required reports; shared acceptance profile applies).

#### BP522.materials.layered_principled — Layering and Principled parameters

Owner: M24 · Scattering + transport + numerical review
Prerequisite milestones: M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory pinned Principled/layered parameters and combinations; implement physically reviewed coat/base/sheen/layer interactions, parameter semantics and limiting cases without name-based equivalence claims.

Evidence: `evidence/parity/blender-5.2.2/materials/layered_principled/` (all seven required reports; shared acceptance profile applies).

#### BP522.materials.volume_transport — Rich participating media

Owner: M25 · Rendering + geometry + animation
Prerequisite milestones: M20, M24, M08
Cross-domain integration contracts: M35
Status: planned_follow_on_scope; qualification not_run

Implement heterogeneous absorption/scattering/emission, phase functions and indirect multiple scattering with volume boundaries, sparse sampling and CPU/GPU-specific capability profiles.

Evidence: `evidence/parity/blender-5.2.2/materials/volume_transport/` (all seven required reports; shared acceptance profile applies).

### Render engines, cameras, lighting and quality — P; comparative performance U

#### BP522.rendering.lens_camera — Camera lenses and depth of field

Owner: M25 · Rendering + geometry + animation
Prerequisite milestones: M20, M24, M08
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Implement perspective/orthographic shifts, depth of field, panoramic models and sensor/lens conventions; validate projection, focus/bokeh and import mappings using matched analytical and image fixtures.

Evidence: `evidence/parity/blender-5.2.2/rendering/lens_camera/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.lights_world — Lights and world sampling

Owner: M25 · Rendering + geometry + animation
Prerequisite milestones: M20, M24, M08
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory light families and world environments; implement normalized emission, world importance sampling and light linking with MIS/energy tests and explicit host/backend support.

Evidence: `evidence/parity/blender-5.2.2/rendering/lights_world/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.shadow_visibility — Shadow and visibility controls

Owner: M25 · Rendering + geometry + animation
Prerequisite milestones: M20, M24, M08
Cross-domain integration contracts: M19
Status: planned_follow_on_scope; qualification not_run

Implement object/ray/light visibility, shadow controls and linking with transparent/volume interactions; compare intended masks and direct/indirect results without silently altering energy semantics.

Evidence: `evidence/parity/blender-5.2.2/rendering/shadow_visibility/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.motion_combinations — Motion blur across domains

Owner: M25 · Rendering + geometry + animation
Prerequisite milestones: M20, M24, M08
Cross-domain integration contracts: M29, M34, M35
Status: planned_follow_on_scope; qualification not_run

Combine camera, transforms, skinned/morph geometry, topology changes, grooms and volumes over shutters with declared sampling/interpolation restrictions and matched temporal scenes.

Evidence: `evidence/parity/blender-5.2.2/rendering/motion_combinations/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.passes_aovs — Render passes and AOVs

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M23, M27, M36
Status: planned_follow_on_scope; qualification not_run

Publish typed pass/AOV inventory with alpha, depth, normals, IDs, light/material contributions and auxiliary denoising data; preserve semantics through compositing and image export.

Evidence: `evidence/parity/blender-5.2.2/rendering/passes_aovs/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.adaptive_sampling — Sampling and convergence controls

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Provide adaptive sampling, light/path controls and documented termination; demonstrate error-versus-time curves and unbiasedness or disclosed bias across fixed difficult scenes.

Evidence: `evidence/parity/blender-5.2.2/rendering/adaptive_sampling/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.denoising — Production denoising

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M29, M36
Status: planned_follow_on_scope; qualification not_run

Qualify spatial/temporal denoising on low-sample sequences with detail retention, motion disocclusion and auxiliary passes; a simple bilateral filter cannot satisfy the full profile.

Evidence: `evidence/parity/blender-5.2.2/rendering/denoising/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.displacement_dicing — Adaptive displacement and geometry quality

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M20, M23
Status: planned_follow_on_scope; qualification not_run

Implement displacement/dicing controls with crack-free boundaries, camera/motion adaptation, memory limits and quality/error measurements; distinguish uniform bounded dicing.

Evidence: `evidence/parity/blender-5.2.2/rendering/displacement_dicing/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.interactive_preview — Eevee-style interactive outcome target

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M23, M40
Status: planned_follow_on_scope; qualification not_run

Define and implement a separately measured real-time preview profile with material/light/shadow/transparency/volume fidelity, progressive edits and temporal stability; do not infer it from final path tracing.

Evidence: `evidence/parity/blender-5.2.2/rendering/interactive_preview/` (all seven required reports; shared acceptance profile applies).

#### BP522.rendering.backend_scaling — Backend/device quality and performance

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M40, M45
Status: planned_follow_on_scope; qualification not_run

Compare independent CPU and each GPU backend with exact scene/settings/hardware and confidence/error bars for quality, latency, throughput and memory; classify unsupported device features explicitly.

Evidence: `evidence/parity/blender-5.2.2/rendering/backend_scaling/` (all seven required reports; shared acceptance profile applies).

### Color and image interchange — Q/P; broad HDR pipeline A

#### BP522.color.working_view — Working, display and view transforms

Owner: M27 · Color science + imaging + IO
Prerequisite milestones: M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Support explicit configurable working/display/view transforms and looks with pinned configuration versions, exposure and gamut behavior; validate numerical reference vectors beyond sRGB/P3 matrices.

Profile-scoped prerequisite only: `rec2020-d65-st2084-absolute-v1` at
`3e3abcdc69e4ebd3b915e6a8204190356de24fba` supplies fixed three-channel linear
coordinate/reference-white/PQ math. See the [contract](color_math.md),
[evidence](../evidence/m27/color-math/verification.md) and
[remaining M27 work](m27_color_math_progress.md). This does not implement the
configurable workflow above or change any full-row status, evidence or axis.

Evidence: `evidence/parity/blender-5.2.2/color/working_view/` (all seven required reports; shared acceptance profile applies).

#### BP522.color.hdr_display — HDR and SDR display pipeline

Owner: M27 · Color science + imaging + IO
Prerequisite milestones: M23
Cross-domain integration contracts: M40
Status: planned_follow_on_scope; qualification not_run

Handle HDR metadata, transfer functions, display capability negotiation, SDR mapping and presentation; verify actual supported HDR/SDR systems and label untested display paths rather than using screenshots as proof.

Evidence: `evidence/parity/blender-5.2.2/color/hdr_display/` (all seven required reports; shared acceptance profile applies).

#### BP522.color.lut_icc — Configuration, LUT and ICC policy

Owner: M27 · Color science + imaging + IO
Prerequisite milestones: M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Define supported color configurations, LUT interpolation/domains and ICC profiles with missing/invalid profile handling, provenance and reproducible round trips.

Evidence: `evidence/parity/blender-5.2.2/color/lut_icc/` (all seven required reports; shared acceptance profile applies).

#### BP522.color.image_formats — Production image interchange

Owner: M27 · Color science + imaging + IO
Prerequisite milestones: M23
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Inventory channel/layer/bit-depth/alpha/metadata and compression support per image format; implement bounded decoding/encoding and scene-linear round trips, including HDR values and malformed inputs.

Evidence: `evidence/parity/blender-5.2.2/color/image_formats/` (all seven required reports; shared acceptance profile applies).

### Animation, rigs, constraints and NLA — Q/P

#### BP522.animation.rig_authoring — Rig, pose, bind and weight tools

Owner: M28 · Animation + numerics + editor
Prerequisite milestones: M18, M08
Cross-domain integration contracts: M19, M30, M41
Status: planned_follow_on_scope; qualification not_run

Construct/edit armatures, rest/bind poses and weights with joint orientation, hierarchy and deformation continuity; preserve authored rigs through transactions, save and interchange.

Evidence: `evidence/parity/blender-5.2.2/animation/rig_authoring/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.constraints — General constraint stacks

Owner: M28 · Animation + numerics + editor
Prerequisite milestones: M18, M08
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory constraints with spaces, ordering, limits and robust dependency-cycle diagnostics; test interacting chains and numerical limits beyond CopyPosition and TwoBoneIk.

Profile-scoped prerequisite only: `PlanarChainIkV1` at
`57d6b02bee76a56f3a17ebace354cc43b8512dc8` supplies snapshot-40 geometry-free planar 3–16-link
direct pose solving and authored-Snapshot JSON reload. See the
[contract](planar_chain_ik_v1.md),
[evidence](../evidence/m28/planar-chain-ik/execution.md) and
[remaining M28 work](m28_planar_chain_ik_progress.md). Aggregate animation rejects
the profile before composition; this does not qualify general constraint stacks,
rig authoring, deformation or any full-row status, evidence or axis.

Evidence: `evidence/parity/blender-5.2.2/animation/constraints/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.dual_quaternion — Deformation breadth

Owner: M28 · Animation + numerics + editor
Prerequisite milestones: M18, M08
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement dual-quaternion and declared nonlinear deformation choices with normalized weights, scale/negative-transform policies and morph/constraint interaction; compare LBS and alternative profiles explicitly.

Evidence: `evidence/parity/blender-5.2.2/animation/dual_quaternion/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.curve_editors — Graph and dope-sheet editing

Owner: M29 · Animation + editor + interchange
Prerequisite milestones: M28, M19
Cross-domain integration contracts: M40
Status: planned_follow_on_scope; qualification not_run

Edit keys, handles, easing, interpolation and extrapolation with rational time, snapping and channel selection; demonstrate UI/API equivalence and lossless save/reopen.

Evidence: `evidence/parity/blender-5.2.2/animation/curve_editors/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.actions_nla — Actions, blending and NLA

Owner: M29 · Animation + editor + interchange
Prerequisite milestones: M28, M19
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Reuse actions in layered tracks/strips with blending, influence, time remap, transitions, extrapolation and mute/solo semantics; verify authored NLA evaluation and export/loss distinctions.

Evidence: `evidence/parity/blender-5.2.2/animation/actions_nla/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.drivers — Animation drivers and dependency evaluation

Owner: M29 · Animation + editor + interchange
Prerequisite milestones: M28, M19
Cross-domain integration contracts: M43
Status: planned_follow_on_scope; qualification not_run

Provide typed driver expressions/data dependencies with deterministic evaluation, invalid-reference/cycle diagnostics, permissions and animated constraint/material/geometry integration.

Evidence: `evidence/parity/blender-5.2.2/animation/drivers/` (all seven required reports; shared acceptance profile applies).

#### BP522.animation.retargeting — Motion retargeting

Owner: M29 · Animation + editor + interchange
Prerequisite milestones: M28, M19
Cross-domain integration contracts: M41
Status: planned_follow_on_scope; qualification not_run

Map rig hierarchies and pose conventions with scale, contact and root-motion policy; quantify fixed retargeted motion errors and retain editable source/target provenance.

Evidence: `evidence/parity/blender-5.2.2/animation/retargeting/` (all seven required reports; shared acceptance profile applies).

### Simulation — Q/P; production families not established

#### BP522.simulation.rotational_contacts — Rigid contacts and collision shapes

Owner: M32 · Physics + collision + numerical review
Prerequisite milestones: M16, M11
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Simulate general supported shapes with rotation, inertia, friction/restitution, broad/narrow-phase collision and stable resting stacks; spheres/floor evidence remains a limited baseline.

Evidence: `evidence/parity/blender-5.2.2/simulation/rotational_contacts/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.joint_families — Rigid joint families

Owner: M32 · Physics + collision + numerical review
Prerequisite milestones: M16, M11
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement declared hinge/slider/cone/locked and motor/limit constraints with breakage policy, numerical stability and invalid configuration diagnostics; distance joints alone do not qualify.

Evidence: `evidence/parity/blender-5.2.2/simulation/joint_families/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.cloth_soft_collision — Cloth and soft-body robustness

Owner: M33 · Deformable physics + collision + animation
Prerequisite milestones: M16, M11, M32
Cross-domain integration contracts: M28
Status: planned_follow_on_scope; qualification not_run

Simulate cloth/volumetric soft bodies with self-collision and moving/deforming colliders, friction, pin/attachment and material behavior; measure penetration, conservation/stability and timestep sensitivity.

Evidence: `evidence/parity/blender-5.2.2/simulation/cloth_soft_collision/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.particle_lifecycle — Particle births and lifecycles

Owner: M34 · Particles + groom + simulation nodes
Prerequisite milestones: M20, M22, M32
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Author deterministic emission, births/deaths, forces, collisions, instancing and time scrubbing with stable identities and cache semantics; fixed finite samples do not satisfy the lifecycle profile.

Evidence: `evidence/parity/blender-5.2.2/simulation/particle_lifecycle/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.groom_dynamics — Groom collisions and surface effects

Owner: M34 · Particles + groom + simulation nodes
Prerequisite milestones: M20, M22, M32
Cross-domain integration contracts: M33
Status: planned_follow_on_scope; qualification not_run

Simulate strand interactions, moving-collider contact and supported surface effects with attach/root semantics, force coupling and render integration; preserve groom authoring and cache identity.

Evidence: `evidence/parity/blender-5.2.2/simulation/groom_dynamics/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.dynamic_paint — Dynamic Paint surface effects

Owner: M34 · Particles + groom + simulation nodes
Prerequisite milestones: M20, M22, M32
Cross-domain integration contracts: M33
Status: planned_follow_on_scope; qualification not_run

Author brush/canvas Dynamic Paint-style color, wetness, displacement and wave surface effects with contact/sampling, drying/diffusion, layer and time/cache policies; verify moving/deforming brushes and surfaces, bake/replay and material/render integration.

Evidence: `evidence/parity/blender-5.2.2/simulation/dynamic_paint/` (all seven required reports; shared acceptance profile applies).

Pinned source: https://github.com/blender/blender/blob/d13f752e3b9c4f8c261cda552b1021f8bcc0382c/source/blender/blenkernel/intern/dynamicpaint.cc

#### BP522.simulation.pressure_smoke — Pressure-projected smoke and fire

Owner: M35 · Fluid numerics + sparse storage + meshing
Prerequisite milestones: M20, M32
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement pressure projection, boundary conditions, buoyancy/advection and combustion with mass/divergence/error measurements on fixed domains; prescribed-flow transport is not equivalent.

Evidence: `evidence/parity/blender-5.2.2/simulation/pressure_smoke/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.free_surface — Free-surface liquid workflows

Owner: M35 · Fluid numerics + sparse storage + meshing
Prerequisite milestones: M20, M32
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Simulate and mesh free surfaces with general moving colliders, sources/drains, viscosity/surface policy and volume conservation; qualify beyond weakly compressible SPH/floor meshing.

Evidence: `evidence/parity/blender-5.2.2/simulation/free_surface/` (all seven required reports; shared acceptance profile applies).

#### BP522.simulation.cache_coupling — Simulation replay and cross-domain coupling

Owner: M35 · Fluid numerics + sparse storage + meshing
Prerequisite milestones: M20, M32
Cross-domain integration contracts: M22, M33, M34
Status: planned_follow_on_scope; qualification not_run

Bake/replay and invalidate coupled rigid/cloth/groom/gas/liquid scenes with clear supported coupling direction, timestep and conservation limits; shared timestamps alone do not establish coupling.

Evidence: `evidence/parity/blender-5.2.2/simulation/cache_coupling/` (all seven required reports; shared acceptance profile applies).

### Compositing and mixed 2D/3D drawing — P/I

#### BP522.compositing.node_breadth — Compositor filters, color, keys and masks

Owner: M36 · Compositing + imaging + scheduling
Prerequisite milestones: M23, M26, M27
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory and implement compositor node families for filters/color/keying/masks with typed domains, alpha/premultiplication and bounds; validate against fixed analytic and shot fixtures beyond current finite operators.

Evidence: `evidence/parity/blender-5.2.2/compositing/node_breadth/` (all seven required reports; shared acceptance profile applies).

#### BP522.compositing.temporal_passes — Temporal and pass-aware compositing

Owner: M36 · Compositing + imaging + scheduling
Prerequisite milestones: M23, M26, M27
Cross-domain integration contracts: M29
Status: planned_follow_on_scope; qualification not_run

Process time-dependent shots and typed render passes with frame dependencies, motion/mask metadata, cache invalidation and exact-time playback/export parity.

Evidence: `evidence/parity/blender-5.2.2/compositing/temporal_passes/` (all seven required reports; shared acceptance profile applies).

#### BP522.compositing.gpu_editor — Compositor devices and interactive editing

Owner: M36 · Compositing + imaging + scheduling
Prerequisite milestones: M23, M26, M27
Cross-domain integration contracts: M40
Status: planned_follow_on_scope; qualification not_run

Define GPU/device fallback policies and matching numerical tolerances; author/debug graphs interactively with preview invalidation, cancellation, resource limits and public API equivalence.

Evidence: `evidence/parity/blender-5.2.2/compositing/gpu_editor/` (all seven required reports; shared acceptance profile applies).

#### BP522.compositing.sequencer_groups — Compositor groups in Sequencer

Owner: M38 · Media codecs + audio + timeline
Prerequisite milestones: M27, M29, M36, M37
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Apply compositor groups inside editorial strips/modifiers with correct strip timing, inputs, color/alpha behavior, caching and final-export equivalence; separate compositor/editorial demos are insufficient.

Evidence: `evidence/parity/blender-5.2.2/compositing/sequencer_groups/` (all seven required reports; shared acceptance profile applies).

#### BP522.compositing.drawing_authoring — Editable layered drawing animation

Owner: M37 · Drawing + animation + editor
Prerequisite milestones: M20, M27, M29
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Edit timed strokes/fills, pressure, brush assets, layers, onion views, keyframes and modifiers with 2D/3D placement, selection and transform semantics; preserve editability through undo/save.

Evidence: `evidence/parity/blender-5.2.2/compositing/drawing_authoring/` (all seven required reports; shared acceptance profile applies).

### Video, audio, tracking and reconstruction — P/I

#### BP522.media_tracking.codecs — Compressed audiovisual codecs and containers

Owner: M38 · Media codecs + audio + timeline
Prerequisite milestones: M27, M29, M36, M37
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory supported containers/codecs and legal/dependency feasibility; implement approved Rust-owned decoding/encoding without hidden non-Rust runtime fallbacks, with malformed-input and round-trip fixtures.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/codecs/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.timestamps_audio — Timestamp, seek and audio correctness

Owner: M38 · Media codecs + audio + timeline
Prerequisite milestones: M27, M29, M36, M37
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Preserve exact frame/sample timing, seek behavior, channel layouts and resampling over variable-rate sources; measure drift, clipping, latency and discontinuities in synchronized playback/export.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/timestamps_audio/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.proxies_streaming — Streaming, proxies and media caches

Owner: M38 · Media codecs + audio + timeline
Prerequisite milestones: M27, M29, M36, M37
Cross-domain integration contracts: M45
Status: planned_follow_on_scope; qualification not_run

Generate/use proxies, stream bounded media and recover/invalidate caches with responsive seeks and fixed long-form memory/latency budgets; test missing media and interrupted cache writes.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/proxies_streaming/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.multitrack_edit — Multi-track editorial workflows

Owner: M38 · Media codecs + audio + timeline
Prerequisite milestones: M27, M29, M36, M37
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Edit video/audio strips, transitions, retiming, effects and mixing through a complete long-form timeline with synchronized playback, undo and encoded export.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/multitrack_edit/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.lens_calibration — Lens calibration and distortion

Owner: M39 · Computer vision + numerics + compositing
Prerequisite milestones: M20, M29, M36, M38
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Estimate and apply camera intrinsics/distortion from fixed calibration footage; report reprojection error, conditioning and unsupported lens models, preserving metadata into 3D integration.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/lens_calibration/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.robust_tracks — Subpixel tracking and outliers

Owner: M39 · Computer vision + numerics + compositing
Prerequisite milestones: M20, M29, M36, M38
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Track features at subpixel precision through illumination change, occlusion and motion with confidence/outlier rejection; report trajectory/error and failure cases beyond integer patches.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/robust_tracks/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.sfm_bundle — Structure from motion and bundle adjustment

Owner: M39 · Computer vision + numerics + compositing
Prerequisite milestones: M20, M29, M36, M38
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Recover cameras and scene structure from uncalibrated/partially calibrated footage with initialization, gauge/scale and bundle adjustment; reject degenerate solves and quantify residuals/uncertainty.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/sfm_bundle/` (all seven required reports; shared acceptance profile applies).

#### BP522.media_tracking.tracked_shot — Synchronized tracked composite

Owner: M39 · Computer vision + numerics + compositing
Prerequisite milestones: M20, M29, M36, M38
Cross-domain integration contracts: M25, M37
Status: planned_follow_on_scope; qualification not_run

Integrate reconstructed camera/geometry, 3D render, drawing, keyed footage and synchronized sound in one editable/exported shot with declared error and recovery budgets.

Evidence: `evidence/parity/blender-5.2.2/media_tracking/tracked_shot/` (all seven required reports; shared acceptance profile applies).

### Scene assembly, assets and production pipeline — Q/P

#### BP522.scene_assets.collections_views — Collections and view layers

Owner: M19 · Document + assets + editor
Prerequisite milestones: M02, M13
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Author hierarchical collections, memberships and per-view visibility with shared geometry, nested policy, transactional conflict handling and save/reopen equivalence; M19-A is a bounded candidate until its required axes qualify.

Post-audit bounded progress: M19-A, M19-CLI, M19-UI-A, M19-UI-A2, M19-INTEGRATION-1, M19-APP-1, M19-EXPORT-1, M19-CLI-INTEGRATION-1, M19-UI-A3, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Native static owner mode now exposes explicit local/captured member transform/material replacements while keeping selection on the authored owner. Source membership/labels and composed values are resolved separately. Source hierarchy/visibility/geometry and broader dynamic/native qualification remain outside scope. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/scene_assets/collections_views/` (all seven required reports; shared acceptance profile applies).

#### BP522.scene_assets.linked_assets — Linked libraries and asset updates

Owner: M19 · Document + assets + editor
Prerequisite milestones: M02, M13
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Link versioned external assets without unintended copies; update/relink dependencies with missing/cycle/version diagnostics, immutable source identity and explicit transaction effects.

Post-audit bounded progress: M19-E, M19-CLI-E, M19-INTEGRATION-1, M19-UI-E, M19-UI-C2, M19-CLI-INTEGRATION-1, M19-SOURCE-1, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Native member art direction now survives compatible preserve-only Refresh. Relink defaults RequireEmpty and can explicitly ClearAll maps after separate all-placement/loss review. This is a bounded loss path, not selective correspondence, automatic recovery or general nested/live library support. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/scene_assets/linked_assets/` (all seven required reports; shared acceptance profile applies).

#### BP522.scene_assets.overrides_variants — Overrides, variants and migrations

Owner: M19 · Document + assets + editor
Prerequisite milestones: M02, M13
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Author local overrides/variants over linked assets and migrate them across upstream changes with conflicts, correspondence/loss reporting, undo and durable provenance.

Post-audit bounded progress: M19-C, M19-D, M19-E, M19-CLI, M19-UI-C, M19-UI-D, M19-CLI-E, M19-INTEGRATION-1, M19-UI-E, M19-UI-C2, M19-EXPORT-1, M19-CLI-INTEGRATION-1, M19-SOURCE-1, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Local/captured member Transform Set/Clear stores a full value replacement; material maps store destination Material IDs resolved from the current destination table. Fresh copies clone values at creation, and Clear returns to the current source. ClearAll Relink explicitly acknowledges all placement losses; selective mapping, sparse live-field overrides and geometry migration remain absent. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/scene_assets/overrides_variants/` (all seven required reports; shared acceptance profile applies).

#### BP522.scene_assets.catalog_packaging — Catalogs, dependency discovery and packaging

Owner: M45 · Integration + performance + operations
Prerequisite milestones: M19, M26, M29, M31, M35, M38, M41, M43
Cross-domain integration contracts: M20
Status: planned_follow_on_scope; qualification not_run

Discover full scene/file dependencies including fonts, textures, media and linked assets; catalog, relink and package reproducibly with licenses/provenance and missing-asset recovery.

Evidence: `evidence/parity/blender-5.2.2/scene_assets/catalog_packaging/` (all seven required reports; shared acceptance profile applies).

#### BP522.scene_assets.pipeline_migration — Asset pipeline and version migration

Owner: M45 · Integration + performance + operations
Prerequisite milestones: M19, M26, M29, M31, M35, M38, M41, M43
Cross-domain integration contracts: M42
Status: planned_follow_on_scope; qualification not_run

Run batch/headless import-author-render-export pipelines across supported document/schema versions with reproducible migration, rollback/recovery and authored-intent checks.

Post-audit bounded progress: M19-CLI, M19-E, M19-CLI-E, M19-INTEGRATION-1, M19-UI-E, M19-APP-1, M19-UI-C2, M19-EXPORT-1, M19-CLI-INTEGRATION-1, M19-SOURCE-1, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Bounded source-free native workflows now include per-placement member replacements, compatible Refresh retention, Clear-to-inherit and separately acknowledged ClearAll Relink with retained assets. Selective intent migration, source geometry editing and production pipeline/recovery qualification remain open. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/scene_assets/pipeline_migration/` (all seven required reports; shared acceptance profile applies).

### Scripting, add-ons and collaboration — Q/P; ecosystem compatibility A/U

#### BP522.extensions.api_sdks — Discoverable API and maintained clients

Owner: M43 · API + sandbox security + developer experience
Prerequisite milestones: M13, M21, M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Cover every domain with versioned schemas, capabilities, bulk/streaming operations and independent clients; verify cancellation, stale revisions and UI/API equivalence without private mutation paths.

Post-audit bounded progress: M19-CLI, M19-E, M19-UI-A, M19-UI-B, M19-UI-C, M19-UI-D, M19-UI-B2, M19-CLI-E, M19-UI-A2, M19-INTEGRATION-1, M19-UI-E, M19-APP-1, M19-UI-C2, M19-EXPORT-1, M19-CLI-INTEGRATION-1, M19-UI-A3, M19-SOURCE-1, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Native narrow member constructors reuse current public local/captured replacement and material commands without core/Host/schema changes. Exact request/peer comparisons do not create authority, and UI bookkeeping remains non-durable intent. Full client/MCP and broader migration contracts remain open. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/extensions/api_sdks/` (all seven required reports; shared acceptance profile applies).

#### BP522.extensions.isolated_runtime — Isolated extension runtime

Owner: M43 · API + sandbox security + developer experience
Prerequisite milestones: M13, M21, M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement explicit permissions, fuel/memory/data limits, compatibility/version policy and independent escape/resource/security review; data-only extensions are not a general isolated runtime.

Evidence: `evidence/parity/blender-5.2.2/extensions/isolated_runtime/` (all seven required reports; shared acceptance profile applies).

#### BP522.extensions.migration_tools — Add-on migration capability

Owner: M43 · API + sandbox security + developer experience
Prerequisite milestones: M13, M21, M23
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory Python/RNA/operator idioms and provide explicit native extension/SDK migration contracts and diagnostics with ported representative workflows; successful migration is not unmodified bpy compatibility.

Evidence: `evidence/parity/blender-5.2.2/extensions/migration_tools/` (all seven required reports; shared acceptance profile applies).

#### BP522.extensions.bpy_compatibility — Actual bpy/RNA/operator compatibility program

Owner: M43 · API + sandbox security + developer experience
Prerequisite milestones: M13, M21, M23
Cross-domain integration contracts: M40, M42
Status: planned_follow_on_scope; qualification not_run

Define versioned bpy/RNA/operator/context coverage and an add-on fixture corpus; resolve Rust-only/security feasibility before implementation, then qualify unmodified supported add-ons separately from native API equivalents. Arbitrary third-party add-ons remain an explicit unbounded extension-policy decision, never silently counted as compatible.

Evidence: `evidence/parity/blender-5.2.2/extensions/bpy_compatibility/` (all seven required reports; shared acceptance profile applies).

Decision: Full bpy behavior versus a finite supported compatibility profile must be explicitly agreed; Rust-only architecture and isolation remain binding. No Python/Blender embedding authorized by roadmap inclusion.

#### BP522.extensions.authenticated_collaboration — Authenticated collaboration product goal

Owner: M44 · Security + collaboration + storage
Prerequisite milestones: M19, M43
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Qualify identities, grants/revocation, tenant isolation, conflicts and reconnect/replay for authorized shared assets; keep standalone local use. Track this as a Render product goal, not an assumed Blender built-in deficit.

Evidence: `evidence/parity/blender-5.2.2/extensions/authenticated_collaboration/` (all seven required reports; shared acceptance profile applies).

Classification: additional Render product goal; excluded from claims about Blender built-in deficits.

### File interoperability and .blend — Q/P; broad edited .blend export A

#### BP522.interchange.format_inventory — Versioned authored format adapters

Owner: M41 · Interchange + security + provenance
Prerequisite milestones: M19, M20, M23, M29, M27, M38
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Inventory OBJ/glTF/USD/Alembic/FBX/PLY/STL and other audited adapter families, versions/extensions and dependencies; separately implement import, authored retention, evaluation, edited export and explicit loss reports.

Evidence: `evidence/parity/blender-5.2.2/interchange/format_inventory/` (all seven required reports; shared acceptance profile applies).

#### BP522.interchange.authored_animation — Authored animation interchange

Owner: M41 · Interchange + security + provenance
Prerequisite milestones: M19, M20, M23, M29, M27, M38
Cross-domain integration contracts: M28
Status: planned_follow_on_scope; qualification not_run

Preserve rigs, skins, morphs, actions and supported time/constraint semantics through declared format round trips; credit bounded animated glTF import while distinguishing evaluated-mesh export.

Evidence: `evidence/parity/blender-5.2.2/interchange/authored_animation/` (all seven required reports; shared acceptance profile applies).

#### BP522.interchange.complex_assets — Materials, media and sparse-volume interchange

Owner: M41 · Interchange + security + provenance
Prerequisite milestones: M19, M20, M23, M29, M27, M38
Cross-domain integration contracts: M35
Status: planned_follow_on_scope; qualification not_run

Round-trip supported material graphs, geometry families, sparse volumes, media/color metadata and external dependencies with finite format feature profiles, malformed inputs and loss-aware conversions.

Evidence: `evidence/parity/blender-5.2.2/interchange/complex_assets/` (all seven required reports; shared acceptance profile applies).

#### BP522.interchange.blend_version_semantics — Broader native blend import and evaluation

Owner: M42 · Native formats + geometry/shading/animation integration
Prerequisite milestones: M17, M21, M23, M28, M29, M41
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Extend beyond uncompressed Blender 2.93 static evaluator to explicitly inventoried versions/compression, modifiers, nodes, rigs/animation and linked libraries; structural parsing alone is not semantic support.

Evidence: `evidence/parity/blender-5.2.2/interchange/blend_version_semantics/` (all seven required reports; shared acceptance profile applies).

#### BP522.interchange.blend_edited_export — Edited native blend export

Owner: M42 · Native formats + geometry/shading/animation integration
Prerequisite milestones: M17, M21, M23, M28, M29, M41
Cross-domain integration contracts: M19
Status: planned_follow_on_scope; qualification not_run

Export the edited Render document as declared Blender versions with authored/evaluated retention and verified reopen/loss reports; extracting original preserved bytes cannot satisfy edited export.

Evidence: `evidence/parity/blender-5.2.2/interchange/blend_edited_export/` (all seven required reports; shared acceptance profile applies).

### Editor UX, accessibility, platforms and reliability — I/U

#### BP522.editor_reliability.editor_workflows — Complete specialized editor workflows

Owner: M40 · Editor + accessibility + platforms
Prerequisite milestones: M09, M15, M19, M21, M23, M29, M30, M31, M36, M37, M38, M39, M22, M32, M33, M34, M35
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Integrate modeling, nodes, rigs, sculpt/paint, assets, simulation, compositor and media with modal tools, gizmos, keymaps, undo and discoverability; verify full task workflows and interrupted/repeated actions.

Post-audit bounded progress: M19-UI-A, M19-UI-B, M19-UI-C, M19-UI-D, M19-UI-B2, M19-UI-A2, M19-UI-E, M19-APP-1, M19-UI-C2, M19-UI-A3, M19-SOURCE-1, M19-LOCAL-INSTANCES-1, M19-CAPTURED-LIBRARIES-1, M19-OWNER-PREVIEW-1, M19-NATIVE-LIBRARIES-1, M19-MEMBER-ART-1. Thirteen new headless member/loss tests plus sixteen unchanged native-library and nine unchanged owner-preview regressions pass. Explicit member choice, scoped replacement edits, cross-intent invalidation and two independent Relink acknowledgements work within the static CPU route; physical and broader source/component tools remain unqualified. Full-row qualification remains not_run; see [graded evidence and remaining work](m19_implementation_progress.md).

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/editor_workflows/` (all seven required reports; shared acceptance profile applies).

#### BP522.editor_reliability.accessibility_input — Accessibility and real input devices

Owner: M40 · Editor + accessibility + platforms
Prerequisite milestones: M09, M15, M19, M21, M23, M29, M30, M31, M36, M37, M38, M39, M22, M32, M33, M34, M35
Cross-domain integration contracts: M10, M14
Status: planned_follow_on_scope; qualification not_run

Qualify keyboard/focus, complex IME, screen readers and physical tablet/pressure/assistive input on declared OS/browser combinations using task/device evidence; compile-only and synthetic events do not count.

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/accessibility_input/` (all seven required reports; shared acceptance profile applies).

#### BP522.editor_reliability.browser_native — Native and browser runtime qualification

Owner: M40 · Editor + accessibility + platforms
Prerequisite milestones: M09, M15, M19, M21, M23, M29, M30, M31, M36, M37, M38, M39, M22, M32, M33, M34, M35
Cross-domain integration contracts: M10, M12, M14
Status: planned_follow_on_scope; qualification not_run

Run declared Windows/macOS/Linux native and live-browser interaction/storage/GPU workflows with actual device/backends; distinguish Node-Wasm, software Vulkan and compile success from runtime qualification.

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/browser_native/` (all seven required reports; shared acceptance profile applies).

#### BP522.editor_reliability.scale_latency — Production-sized workload tiers

Owner: M45 · Integration + performance + operations
Prerequisite milestones: M19, M26, M29, M31, M35, M38, M41, M43
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Freeze small/medium/large assets and long-form shots, measure cold load/save, edit/brush/graph/scrub/render latency, throughput and RAM/VRAM/storage with uncertainty and fixed budgets.

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/scale_latency/` (all seven required reports; shared acceptance profile applies).

#### BP522.editor_reliability.recovery — Reliability and operational recovery

Owner: M45 · Integration + performance + operations
Prerequisite milestones: M19, M26, M29, M31, M35, M38, M41, M43
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Test interrupted writes/exports, full storage, corrupt roots, process crashes, device loss and resource pressure without weakening transaction/ownership guarantees; provide recovery workflows and diagnostics.

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/recovery/` (all seven required reports; shared acceptance profile applies).

#### BP522.editor_reliability.qualification_inventory — Evolving full-parity inventory and release gate

Owner: M46 · Release + all subsystem maintainers
Prerequisite milestones: M09, M10, M12, M14, M15, M16, M17, M18, M19, M20, M21, M22, M23, M24, M25, M26, M27, M28, M29, M30, M31, M32, M33, M34, M35, M36, M37, M38, M39, M40, M41, M42, M43, M44, M45
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Reconcile the pinned source inventory at semantic depth, expand newly found behavior into tracked rows, qualify every agreed axis/backend and integration workflow, retain open original release gates and obtain actual owner/release sign-off; never infer parity percentage from counts.

Evidence: `evidence/parity/blender-5.2.2/editor_reliability/qualification_inventory/` (all seven required reports; shared acceptance profile applies).

### 3. What the expanded plan should make explicit (item 5)

#### BP522.npr.freestyle_lines — Non-photoreal line rendering

Owner: M26 · Graphics + performance + verification
Prerequisite milestones: M24, M25
Cross-domain integration contracts: M20, M23, M37
Status: planned_follow_on_scope; qualification not_run

Implement a named independent line-rendering profile covering feature-edge detection, visibility/occlusion, chaining and stylized strokes with temporal coherence and line/pass export; measure against selected Freestyle outcomes.

Evidence: `evidence/parity/blender-5.2.2/npr/freestyle_lines/` (all seven required reports; shared acceptance profile applies).

#### BP522.npr.toon_materials — Stylized and toon shading

Owner: M23 · Shading + GPU + editor
Prerequisite milestones: M07, M19, M21
Cross-domain integration contracts: none beyond prerequisites
Status: planned_follow_on_scope; qualification not_run

Implement controllable toon/stylized lighting, ramps and outlines with declared light/material semantics and reproducible animated fixtures; qualify independently of physically based shading.

Evidence: `evidence/parity/blender-5.2.2/npr/toon_materials/` (all seven required reports; shared acceptance profile applies).

#### BP522.npr.drawing_render_integration — NPR and Grease Pencil integration

Owner: M37 · Drawing + animation + editor
Prerequisite milestones: M20, M27, M29
Cross-domain integration contracts: M23, M26, M36
Status: planned_follow_on_scope; qualification not_run

Combine editable drawing layers, toon materials and line-render passes with 3D depth, lighting and compositing across animation; demonstrate one coherent authored/rendered shot.

Evidence: `evidence/parity/blender-5.2.2/npr/drawing_render_integration/` (all seven required reports; shared acceptance profile applies).

## Eight explicit audit refinements

1. Geometry-node bundles/lists and grid/SDF fields, sound-driven geometry and node XPBD: BP522.nodes.bundles_lists, BP522.nodes.grid_sdf, BP522.nodes.sound_geometry, BP522.nodes.xpbd_solver

2. Stateful invalidation, bake/replay, time and tool context: BP522.nodes.zones_time, BP522.nodes.cache_bake, BP522.nodes.node_tools

3. Compositor groups in editorial strips/modifiers: BP522.compositing.sequencer_groups

4. Interactive renderer outcome distinct from final tracing: BP522.rendering.interactive_preview

5. NPR/Freestyle, toon materials and drawing integration: BP522.npr.freestyle_lines, BP522.npr.toon_materials, BP522.npr.drawing_render_integration

6. Dependencies, libraries, overrides, migration and round-trip losses: BP522.scene_assets.linked_assets, BP522.scene_assets.overrides_variants, BP522.scene_assets.catalog_packaging, BP522.scene_assets.pipeline_migration, BP522.interchange.format_inventory

7. Add-on/API compatibility scope decision: BP522.extensions.api_sdks, BP522.extensions.migration_tools, BP522.extensions.bpy_compatibility

8. HDR/SDR managed display and outputs: BP522.color.working_view, BP522.color.hdr_display, BP522.color.lut_icc, BP522.color.image_formats

## Inventory evolution and release

Maintain immutable baseline/version history; new source-discovered behaviors or newer Blender releases add rows and explicit migration decisions. Do not silently move the target or erase failed/blocked requirements.

A full-parity claim requires all agreed applicable rows/axes qualified against the published versioned inventory, reviewed differences/constraints and complete cross-domain/release evidence; counts are not percentages. Product-only goals are tracked separately.

This is complete coverage of the captured audit gap list, not evidence that the audit discovered every Blender operator, parameter or behavior. Expand source-level operator/node/format inventories and integration fixtures within these rows, then split new independently meaningful gaps into stable IDs. Keep prior baseline results and differences visible.

No Blender code was copied or translated. No tests, render engines, browsers or benchmarks were executed to claim these requirements pass. No remote publication is implied.


## Documentation validation

The initial-admission checker verifies 47 unchanged historical milestone records, 104 unique owned requirements, all 16 audited domain excerpts plus NPR, all eight refinement mappings and seven separate axes per row. Ten deliberately corrupted documentation variants are rejected. The prior append-only contract check and its nine negative documentation tests also pass. These are text/traceability checks, not product-runtime tests or full discovery of every Blender behavior. See `planning/blender_parity_review.json` and `scripts/check-blender-parity-roadmap.py`.

Independent audit coverage review found no blocking omissions. The review tightened the surface-effects scope with an explicit Dynamic Paint requirement. It confirmed the qualified/candidate, animated-import/evaluated-export, preserved-bytes/edited-export and native-migration/bpy-compatibility distinctions.
