# Blender 5.2.2 versus Render: source-grounded parity audit

Prepared 7 October 2026. Read-only audit; neither Blender nor Render was built or executed for this comparison.

## Bottom line

Render has meaningful independent foundations across most creative domains, but its implemented breadth is much smaller than the family names suggest. The largest remaining differences are general-purpose algorithms, artist-facing workflows, cross-domain integration, format compatibility, and qualification at realistic scale. Having a cloth solver, a node graph, or a bevel operation does not establish Blender-equivalent cloth, Geometry Nodes, or beveling.

The expanded 47-milestone plan is already a good high-level map: 11 completed, 6 in progress, 30 planned. These are project-status counts, **not a percentage of Blender parity**. The most useful improvement is a versioned, feature-level acceptance matrix underneath the plan, including explicit Blender 5.2 details and separate implementation/qualification axes.

Recommended immediate direction: continue dependency-ready scene assembly (M19), keep the advanced modeling track moving independently, and freeze the comparison matrix now. Do not make M19 depend on unfinished C2 cleanup work when its declared dependencies M02 and M13 are already complete.

## 1. Exact comparison baseline and confidence

### Blender

- Official `blender/blender` repository, tag **v5.2.2**, commit **d13f752e3b9c4f8c261cda552b1021f8bcc0382c**
- Commit timestamp: 14 September 2026, 17:14:06 +02:00; release announced 15 September 2026
- The full main source checkout was downloaded, not merely a feature list. Dependency binaries/submodules were not downloaded or executed
- [Pinned source](https://github.com/blender/blender/tree/d13f752e3b9c4f8c261cda552b1021f8bcc0382c), [official release history](https://www.blender.org/releases/), [5.2 manual](https://docs.blender.org/manual/en/5.2/index.html)

### Render

- Qualified checkpoint **7fff667deb967728c56d539b48361323502143aa**, dated 2 October 2026, restored through the full bundle and sequential C1b, C1c, and combined C1d/C1e incremental bundles in recovery v75
- Expanded roadmap: `planning/milestones.json`; the recovered history includes roadmap commit **3cb99091ca4f06c331390425cba5f961e149d1f4**
- Additional `candidate/` recovery files were inspected as a separate C2 overlay. They are **implemented candidate work, not newly qualified capability**
- References below are repository-relative source and contract paths from that recovered checkpoint. They do not assume the public GitHub branch contains these later commits
- An M19 implementation started separately after this baseline; it is not included as completed capability in this audit

### Status vocabulary

- **Qualified, limited scope (Q):** historical qualification is recorded for a named finite profile. This audit inspected the contract/source; it did not rerun those tests
- **Partial (P):** some relevant implementation exists, with material semantic or workflow gaps
- **Implemented, not fully qualified (I):** candidate functionality exists but its required qualification is still open
- **Absent from inspected profile (A):** explicit contract exclusions or closed typed implementation inventory show the capability is not available in this baseline
- **Unknown (U):** evidence is insufficient. No positive claim follows from a module name or an empty search

Statuses apply to individual capabilities, not an entire domain. Historical qualification does not automatically carry over to modified candidate bytes, every GPU, every operating system, or every input size.

## 2. Domain comparison

### Modeling, topology, modifiers — Q/P; general bevel/Boolean A

Render already has stable-ID geometry, correspondence/loss reporting, planar polygon admission, primitives, region work, triangulation, winding/normals, weld/repair and progressively richer cleanup. Its source explicitly distinguishes `BevelBox` and `BooleanBox` from general mesh operations. Earlier extrusion/inset/subdivision profiles are bounded; recent M15/M16 progress must be credited rather than repeating only the original box demo.

Blender's BMesh editing and modifier stack cover much broader selection, cutting, filling, beveling, Boolean intersection, subdivision and deformation workflows. The gap is robust treatment of arbitrary admissible topology, UV/material/normal transfer, modifier ordering, and interactive edit continuity, not simply adding more operation names.

Render pointers: `crates/core/src/modeling.rs` (`Operation`, `BevelBox`, `BooleanBox`); `modeling/selection.rs`, `region.rs`, `cut.rs`, `repair.rs`, `cleanup/`; `docs/m06_modeling_contract.md`, `modeling_regions.md`, `modeling_cleanup_apply.md`. Roadmap: M15–M18. Blender pointers: [BMesh bevel](https://github.com/blender/blender/blob/v5.2.2/source/blender/bmesh/tools/bmesh_bevel.cc), [Boolean modifier](https://github.com/blender/blender/blob/v5.2.2/source/blender/modifiers/intern/MOD_boolean.cc), [subdivision modifier](https://github.com/blender/blender/blob/v5.2.2/source/blender/modifiers/intern/MOD_subsurf.cc).

### Sculpt, remeshing and retopology — P/I

Render supports draw/smooth/flatten, masks/face sets, sparse displacement, recovery/undo and retopology snapping. Its multiresolution profile uses triangle-centroid prolongation; dynamic topology splits selected triangles; remesh is explicitly lossy vertex clustering. Those are useful implementations, but they are not general voxel remeshing, Blender multiresolution, a complete brush system, or robust adaptive topology.

Missing depth includes brush assets and stroke behavior, automasking, topology/pose/cloth-aware brushes, attribute-preserving remesh policy, evaluated-surface snapping, and sustained practical latency with dense assets. Physical tablet/assistive-technology evidence is still open, separate from engine correctness.

Render: `crates/core/src/sculpt/`, `docs/sculpt_workflows.md`, `sculpt_displacements.md`; M10/M31/M40. Blender: [sculpt/paint source](https://github.com/blender/blender/tree/v5.2.2/source/blender/editors/sculpt_paint), [official sculpt introduction](https://docs.blender.org/manual/en/latest/sculpt_paint/sculpting/introduction/index.html). The current manual distinguishes voxel remeshing, Dyntopo and multiresolution; the comparison must preserve those separate workflows.

### UV, texture/vertex/weight paint and baking — Q/P/I

Render has real seam-aware UV authoring, pinned constraints, rectangular atlas packing, tiled/projected paint and named bake outputs. Its disk-chart solver, rectangular packing, center-dab/discontinuity limits and finite bake set remain significant restrictions. A texture-colored output does not prove full paint authoring or material-aware baking.

Needed: broader UV topology, chart packing/rotation/pinning strategies, multi-tile workflows, clone/smear/brush texture behavior, vertex and weight editing, cage/ray-distance control, material-aware bake passes and dilation, with undo/recovery across long strokes.

Render: `crates/core/src/uv/`, `painting/`, `docs/uv_authoring.md`, `tiled_painting.md`, `sculpt_workflows.md`; M10/M30. Blender: [UV parametrizer](https://github.com/blender/blender/blob/v5.2.2/source/blender/geometry/intern/uv_parametrizer.cc), [UV editor](https://github.com/blender/blender/tree/v5.2.2/source/blender/editors/uvedit).

### Curves, surfaces, text and implicit geometry — Q/P; general authoring A

Render curves/points/grooms are authored and evaluated, mainly into bounded polygon sweeps. This is more than a placeholder, but not equivalent to all curve/surface/text/metaball editing or analytic render representations. General editable shaped text, parametric surfaces and implicit authoring are explicitly outside the current profile.

Render: `curves.rs`, `groom.rs`, `hair_import.rs`, `docs/m07_m08.md`; M20. Blender: `source/blender/editors/curve`, `curves`, `metaball`, `lattice`, and `blenkernel`. Separate geometry representation, editing tools and render evaluation in the matrix.

### Geometry Nodes and procedural execution — Q/P; stateful breadth A

The inspected Render `procedural::Node` inventory is concrete: input/mesh/box/primitive, modify/group, set positions, position/scalar/vector, add/scale/component/sine. It is a small typed point-field graph reusing direct modeling implementations. It is not a broad multi-domain Geometry Nodes system.

Beyond fields and instances, Blender 5.2.2 contains simulation/repeat zones, grid/SDF operations, bundle sockets, audio-frequency sampling and an XPBD solver node with rotational state, collisions and friction. These are explicit extra rows to add under M21/M22/M32–M35/M38, not capabilities implied by their titles.

Render: `crates/core/src/procedural.rs`; M21/M22. Blender: [geometry nodes](https://github.com/blender/blender/tree/v5.2.2/source/blender/nodes/geometry/nodes), especially `node_geo_simulation.cc`, `node_geo_repeat.cc`, `node_geo_xpbd_solver.cc`, `node_geo_sample_sound_frequencies.cc`, `node_geo_sdf_grid_boolean.cc`, and `node_geo_grid_advect.cc`. [Simulation-zone manual](https://docs.blender.org/manual/en/5.2/modeling/geometry_nodes/simulation/simulation_zone.html) also documents state/caching and context restrictions; parity must include these semantics, not only a loop node.

### Materials and path transport — Q/P; full shader graphs A

Render has bounded PBR, ideal dielectric, conductor GGX, a single-interface coat, alpha, textured surfaces, displacement, sparse media and temporal rendering. The `scattering::Model` enum and backend contracts make the limits explicit. CPU support and GPU support differ; GPU volume support is narrower.

Blender adds broad shader graphs, texture/coordinate/procedural nodes, rough transmission, subsurface transport, physical hair models, richer layering and volume scattering. Render grooms rendered as sweep surfaces are not physical fiber scattering. A model named Principled is not automatically the same model or all parameters of Blender Principled.

Render: `scattering.rs`, `pbr.rs`, `textures.rs`, `volumes.rs`, `displacement.rs`, `docs/m07_m08.md`, `gpu_media.md`, `gpu_surfaces.md`; M23–M25. Blender: [Cycles closures](https://github.com/blender/blender/tree/v5.2.2/intern/cycles/kernel/closure), including `bssrdf.h`, `bsdf_microfacet.h`, the Chiang/Huang hair models, volume phase functions; [shader nodes](https://github.com/blender/blender/tree/v5.2.2/source/blender/nodes/shader).

### Render engines, cameras, lighting and quality — P; comparative performance U

Render's CPU/GPU transport and approximate raster preview are meaningful independent engines. They should not be labeled Cycles/Eevee parity. Camera source currently exposes perspective and orthographic lenses; the imported Blender profile rejects shift and depth of field. Finite sampling/depth, uniform displacement dicing and simple bilateral filtering do not establish production convergence or denoising quality.

Matrix rows need lens/DOF/panoramic behavior, lights and light linking, world importance sampling, shadow/visibility controls, indirect volume transport, passes/AOVs, denoising, adaptive sampling, motion blur combinations and interactive preview fidelity. Exact supported subfeatures must be inspected before each claim.

Render: `cameras.rs`, `render.rs`, GPU crates, `products.rs`, `docs/multibounce_pbr.md`, `gpu_shutter.md`; M25/M26. Blender: [Cycles integrator](https://github.com/blender/blender/tree/v5.2.2/intern/cycles/kernel/integrator), [Eevee](https://github.com/blender/blender/tree/v5.2.2/source/blender/draw/engines/eevee). Official [Cycles settings](https://docs.blender.org/manual/en/5.2/render/cycles/render_settings/index.html) distinguish CPU/GPU and OSL/backend restrictions; Blender itself is not feature-uniform across devices.

### Color and image interchange — Q/P; broad HDR pipeline A

Render declares scene-linear sRGB, D65 sRGB/Display-P3 matrices, exposure/Reinhard and a spatial bilateral filter. That is a finite color pipeline, not broad configurable color management. Needed: explicit working/display/view transforms, HDR metadata/display handling, configuration/LUT/ICC policy and production image-format support, with numeric vectors and round trips.

Render: `products.rs`, `imaging.rs`, `docs/m07_m08.md`; M27. Blender: [color management implementation](https://github.com/blender/blender/blob/v5.2.2/source/blender/imbuf/intern/colormanagement.cc), `release/datafiles/colormanagement/config.ocio`; [5.2 system-color documentation](https://docs.blender.org/manual/en/5.2/render/color_management/system_configuration.html). Matching a screenshot on one SDR monitor is insufficient.

### Animation, rigs, constraints and NLA — Q/P

Render has rational time, interpolation, clips, LBS skinning, morphs, bind/rest semantics and sampled shutters. Animated glTF import exists and must not be incorrectly described as static-only import. The inspected constraint inventory is `CopyPosition` plus `TwoBoneIk`; broad rig construction, constraints/drivers, dual-quaternion deformation and retargeting are not established.

Needed: pose/weight/bind tools, graph/dope-sheet editing, handles/easing/extrapolation, action reuse/blending/NLA, drivers, constraint spaces/order and robust cyclic-dependency diagnostics. Format export needs authored animation, not just evaluated mesh snapshots.

Render: `animation.rs`, `rigging.rs`, `docs/m07_m08.md`, `animated_gltf.md`, `morph_frames.md`; M28/M29/M41. Blender: [constraints](https://github.com/blender/blender/blob/v5.2.2/source/blender/blenkernel/intern/constraint.cc), `source/blender/animrig`, `editors/space_graph`, `space_action`, `space_nla`; [NLA tracks manual](https://docs.blender.org/manual/en/5.2/editors/nla/tracks.html).

### Simulation — Q/P; production families not established

M11's completed status is valid for its seven named numerical models. It does not mean Blender-level general simulation:

- Rigid bodies: frictionless spheres/floor and distance joints; missing general rotational contacts, collision shapes, friction and joint families
- Cloth/soft body: small XPBD membranes/tetrahedra; missing robust self-collision and general moving colliders
- Particles/grooms: explicit finite samples/strands; missing broad births/lifecycles, groom collisions and surface effects
- Smoke/fire: prescribed-flow transport/reaction; not pressure-projected fluid motion
- Liquid: weakly compressible SPH and floor meshing; not a general free-surface/collider workflow

Render: `crates/core/src/simulation/`, `docs/simulation.md`; M32–M35. Blender: [rigid body integration](https://github.com/blender/blender/blob/v5.2.2/source/blender/blenkernel/intern/rigidbody.cc), [fluid integration](https://github.com/blender/blender/blob/v5.2.2/source/blender/blenkernel/intern/fluid.cc), `intern/mantaflow`, `intern/rigidbody`, and the newer XPBD geometry infrastructure. Cache/replay and cross-domain coupling need separate acceptance, not inference from shared timestamps.

### Compositing and mixed 2D/3D drawing — P/I

Render's compositor implements source/mask/grade/key/over/add/translation/box blur/global normalization with real typed ports and bounded scheduling. Drawing retains timed strokes/fills, pressure and onion views. These are legitimate narrow workflows, not broad compositor or Grease Pencil parity.

Needed: broad filters/color/keying/masks, temporal processing, pass-aware compositing, GPU/device policies, interactive node editing, editable stroke/brush/layer/timeline/modifier semantics and render integration. Blender 5.2 additionally integrates compositor groups into Sequencer strips/modifiers, so standalone domain tests are not sufficient.

Render: `compositor.rs`, `drawing.rs`, `docs/m12_workspaces.md`; M36/M37. Blender: [compositor](https://github.com/blender/blender/tree/v5.2.2/source/blender/compositor), [Grease Pencil editing](https://github.com/blender/blender/tree/v5.2.2/source/blender/editors/grease_pencil), [compositor usage](https://docs.blender.org/manual/en/5.2/compositing/usage.html).

### Video, audio, tracking and reconstruction — P/I

Render assembles image/audio sequences and exposes PCM16 WAV, PPM/PFM output, with other named image inputs. This is not compressed audiovisual container/codec support, streaming playback or a long-form editor. Tracking uses a bounded integer-patch method and calibrated known-control pose/reconstruction, not a general footage solve.

Needed: codec/container feasibility and licensing, timestamp/seek/audio-resampling correctness, proxies/cache, synchronized playback, multi-track editing, lens calibration/distortion, robust subpixel tracks, outlier rejection, structure from motion and bundle adjustment.

Render: `media.rs`, `tracking.rs`, `sequence.rs`, `docs/m12_workspaces.md`; M38/M39. Blender: [Sequencer](https://github.com/blender/blender/tree/v5.2.2/source/blender/sequencer), [tracking](https://github.com/blender/blender/blob/v5.2.2/source/blender/blenkernel/intern/tracking.cc), `intern/libmv`, `editors/space_clip`, `blenkernel/intern/sound*`.

### Scene assembly, assets and production pipeline — Q/P

Render has shared typed assets, immutable content identities, transforms/layers, durable document operations and local branch/merge foundations. Rich collection/view-layer membership and visibility, linked asset update/override policy, catalogs, relink/package workflows and production asset provenance are incomplete.

M19 is an appropriate independent next slice. A useful first result is hierarchical collections plus view-layer visibility with shared geometry preserved, transaction conflicts tested, and save/reopen equivalence. Linked libraries and override migration should follow as separate subgates rather than being silently included in that first slice.

Render: `document.rs`, `storage.rs`, `collaboration.rs`, `docs/workspace_assets.md`; M19/M45. Blender: `source/blender/asset_system`, `editors/asset`, [library overrides](https://github.com/blender/blender/blob/v5.2.2/source/blender/blenkernel/intern/lib_override.cc), `blenkernel/intern/collection.cc`, `layer.cc`.

### Scripting, add-ons and collaboration — Q/P; ecosystem compatibility A/U

Render's public transactions, narrow ABI and independent clients are a strength. Its bounded data-only extensions and trusted local scoped sessions are not a full isolated plugin runtime or deployed authenticated collaboration service. Blender's Python/RNA/operator ecosystem is a separate compatibility target; matching outcomes through a Rust API does not make existing `bpy` add-ons run.

M43 should state whether the goal is native extension capability, migration tooling, or actual Blender add-on compatibility. The current plan explicitly does not imply `bpy` compatibility. M44 collaboration is a useful Render product goal, not a deficit against an assumed built-in Blender multi-user cloud service.

Render: `api.rs`, `agent.rs`, `collaboration.rs`, `docs/api_and_agent_contracts.md`, `collaboration.md`; M43/M44. Blender: [Python integration](https://github.com/blender/blender/tree/v5.2.2/source/blender/python), `makesrna`, `windowmanager`, `scripts/modules`.

### File interoperability and .blend — Q/P; broad edited .blend export A

Render supports named OBJ/glTF subsets, including a bounded animated glTF importer and evaluated exports. Its `.blend` evaluator is specifically **uncompressed Blender 2.93 static scenes**, despite a structural reader recognizing a wider version range. It rejects active modifiers, rigs/animation, linked libraries and unsupported node dependencies. Extracting preserved original bytes is not exporting the edited native document as `.blend`.

M41/M42 need separate import, authored retention, evaluation, edited export and loss-report axes for each version/format. Blender source includes USD, Alembic, FBX, OBJ, PLY, STL and other adapters; each brings semantic and dependency work, not just file extensions.

Render: `blend/`, `interchange.rs`, `gltf_scene.rs`, `gltf_export.rs`, `docs/blend_static.md`, `animated_gltf.md`; M41/M42. Blender: [format adapters](https://github.com/blender/blender/tree/v5.2.2/source/blender/io), [native loader](https://github.com/blender/blender/tree/v5.2.2/source/blender/blenloader).

### Editor UX, accessibility, platforms and reliability — I/U

Render has shared controllers and native/window/browser implementations, plus historical sampled native-window evidence. Full browser interaction/storage, complex IME, screen-reader, physical tablet and other required runtime gates remain open. Native compile success, software Vulkan and Node/Wasm are not physical-GPU or live-browser qualification.

Blender has specialized editors, modal tools, configurable keymaps, gizmos, undo, asset browsers and extensive host integration. Neither application's accessibility should be declared universally successful without task/device testing. Render needs workflow completeness and latency/recovery evidence alongside algorithm correctness.

Render: `editor/`, studio host, `docs/release_qualification.md`, `input_support.md`; M09/M14/M40/M45/M46. Blender: [editors](https://github.com/blender/blender/tree/v5.2.2/source/blender/editors), `windowmanager`, `intern/ghost`, [Cycles backend source](https://github.com/blender/blender/tree/v5.2.2/intern/cycles/kernel/device). No speed, memory-efficiency or production-readiness winner was measured in this audit.

## 3. What the expanded plan should make explicit

The broad plan already anticipates most gaps above. Source inspection adds concrete acceptance rows that family headings can conceal:

1. Geometry-node bundles/lists and grid/SDF fields, plus sound-driven geometry and node-based XPBD physics
2. Stateful cache invalidation, bake/replay, simulation-zone time semantics and modifier-versus-tool context limits
3. Compositor groups inside editorial strips/modifiers, with exact-time and color behavior preserved
4. Eevee-style interactive rendering as its own outcome target, distinct from path-tracing final output
5. Non-photoreal line rendering/Freestyle, stylized/toon materials and Grease Pencil integration; source exists under `source/blender/freestyle` and Cycles toon closures, but this audit has not found a fully specified corresponding Render acceptance profile
6. File/asset dependency discovery, linked-library overrides, version migration and round-trip losses as user workflows
7. Add-on/API compatibility as an explicit scope decision rather than an assumed consequence of functional parity
8. Color-managed display/output across HDR/SDR systems, not only conversion matrices

These are proposed refinements, not edits made to the roadmap. Some are covered conceptually already; each still needs a finite inventory and executable contract.

## 4. Independently implementable stages

| Stage | Concrete deliverable | Dependency/qualification boundary |
|---|---|---|
| A: Frozen parity ledger | Enumerate supported Blender 5.2.2 capability rows, source references, negative cases and fixtures | Documentation work can proceed now; no execution required |
| B: Scene assembly | Collections, per-view visibility, shared-instance preservation, then linked assets/overrides | M19 depends on completed M02/M13; independent of C2 cleanup |
| C: General asset modeling | Complete M15/M16 subgates; general bevel/Boolean; reusable subdivision/deformation policies | Respect geometry/attribute contracts; don't waive C2's unfinished qualification |
| D: Procedural substrate | Multi-domain fields/instances, typed ports/groups, then stateful zones/grids/tools | Split stateless M21 from time/cache-sensitive M22 |
| E: Shading and imaging | Shader graphs, advanced scattering, light/camera transport, color and outputs | M23–M27 can be sliced; CPU and each GPU profile remain separate |
| F: Character/asset creation | Rig/deformation + animation editing; richer UV/paint/bake; sculpt/retopo | Parallel subtracks M28–M31, joined by a complete animated textured asset |
| G: Dynamics | Rotational contacts, robust cloth/soft body, groom/particles, pressure-projected gas/liquid | M32–M35 independently qualify physical models before coupled scenes |
| H: Shot finishing | Compositor/drawing, audiovisual codecs/editing, robust tracking | M36–M39; require a synchronized tracked composite, not separate demos |
| I: Production gate | Editor/platform qualification, authored interchange, extension security, large-workload/recovery corpus | M40–M46 plus original M09/M10/M12/M14 gates; release requires ownership/sign-off |

Do not assign calendar estimates until algorithm spikes, fixtures, workload sizes and engineering capacity are known. Useful parallelism comes from clear contracts, not counting milestones as equal effort.

## 5. Definition of eventual “100% parity”

Use a finite baseline: **Blender 5.2.2 application capabilities under a published inventory**, with an explicit policy for optional extensions, legacy behavior and later releases. If the ambition includes every third-party add-on or arbitrary historic `.blend`, treat those as separate enormous compatibility programs; they cannot be implied by the current roadmap.

For every row record:

- Capability ID, Blender version/source, exact semantics, defaults and unsupported cases
- Render implementation/source identity, dependency milestone and owner
- Separate outcomes for **authoring, evaluation, final rendering, preview, human UI, import and edited export**
- Separate native OS/device/backend/browser evidence; pass/fail/blocked/not run
- Independent positive, adversarial, numerical/property and migration fixtures
- Matched reference scenes/settings, error tolerances, latency/memory budgets and recovery behavior
- Whether differences are deliberate product choices, temporary gaps, or incompatibilities

Example: `mesh.boolean.difference.attributed.nonconvex` must test actual nonconvex attributed input, UV/material correspondence, invalid/degenerate cases, cancel/stale/recovery, save/reopen and UI/API equivalence. Passing `BooleanBox` does not satisfy it.

Example: `rig.constraint.two_bone_ik` may qualify for its precise analytical profile while `rig.constraint.general_stack` remains absent or partial. Both statements can be true simultaneously.

The finishing condition is all agreed rows qualified, with acceptable workflow fidelity and measured constraints. Do not convert milestone totals, code size, test counts or node counts into a feature-parity percentage. Maintain later Blender releases as new baselines rather than moving the target invisibly.

## 6. Licensing and implementation boundary

Blender's source is GPL-licensed; its repository also records dependency-specific licenses. Inspection of behavior and interfaces is useful research, but copying or translating Blender implementation into Render is a separate licensing decision. This audit copied no Blender implementation into Render and made no product-source changes.

For Render's independent Rust implementation, use independently written designs, public specifications and mathematical references, original fixtures with clear provenance, and documented semantic tests. Do not treat a language rewrite as an automatic removal of license obligations. Any proposed code reuse or distribution arrangement should receive a specific license review. This is an engineering boundary, not legal advice. See [Blender licensing FAQ](https://www.blender.org/support/faq/) and [pinned COPYING](https://github.com/blender/blender/blob/v5.2.2/COPYING).

## 7. Audit limitations and next decision

This was a source/contract audit, not exhaustive verification of every Blender operator or every Render execution path. No new tests, benchmark claims, GPU claims, production sign-off or compatibility percentage are asserted. Historical evidence was used only within its declared scope; denied or blocked runtime qualification was not resumed.

The concrete next step is a small M19 collection/view-layer workflow plus the parity ledger, while modeling and other dependency-ready subgates continue independently. The long-term goal is credible if progress is tracked at semantic/workflow depth and each release states exactly what is qualified.

## Appendix: recovered milestone status inventory

These are recorded roadmap states, not new qualification results.

| ID | Status | Scope |
|---|---|---|
| M00 | completed | Architecture and Rust-only feasibility spikes |
| M01 | completed | Document and geometry core |
| M02 | completed | Transactions, API/ABI alpha and durable jobs |
| M03 | completed | CPU renderer and initial native interchange |
| M04 | completed | Portable GPU renderer and browser-local execution |
| M05 | completed | Agent-rendering alpha and narrow ABI v1 |
| M06 | completed | Modeling kernel and procedural geometry |
| M07 | completed | Materials, rendering breadth and geometry families |
| M08 | completed | Animation, rigs and deformation |
| M09 | in_progress | Shared interactive editor |
| M10 | in_progress | UVs, painting, sculpting and retopology |
| M11 | completed | Simulation families |
| M12 | in_progress | Compositing, drawing, media and tracking |
| M13 | completed | Interoperability, collaboration and extension hardening |
| M14 | in_progress | Major-suite conformance and production readiness |
| M15 | in_progress | General polygon selection and region authoring |
| M16 | in_progress | Mesh construction, cutting and topology repair |
| M17 | planned | General bevel and mesh Boolean operations |
| M18 | planned | Iterative subdivision and deformation modifiers |
| M19 | planned | Scene assembly, linked assets and variants |
| M20 | planned | Curves, surfaces, text and implicit geometry authoring |
| M21 | planned | Domain-aware procedural geometry and field breadth |
| M22 | planned | Stateful geometry nodes and interactive node tools |
| M23 | planned | Typed shader graphs and texture authoring |
| M24 | planned | Advanced surface, fiber and layered scattering |
| M25 | planned | Camera, lighting, volume and motion transport depth |
| M26 | planned | Render quality controls and measured backend scaling |
| M27 | planned | HDR color management and image interchange |
| M28 | planned | Rig construction, constraints and deformation depth |
| M29 | planned | Animation editing, blending and retargeting |
| M30 | planned | General UVs, painting and material-aware baking |
| M31 | planned | Production sculpt, remeshing and retopology |
| M32 | planned | Rotational rigid bodies, collision and joints |
| M33 | planned | Cloth and soft-body collision robustness |
| M34 | planned | Particle, groom and surface-effect dynamics |
| M35 | planned | Pressure-projected smoke and free-surface liquids |
| M36 | planned | Compositing graphs and temporal shot finishing |
| M37 | planned | Editable mixed 2D and 3D drawing animation |
| M38 | planned | Compressed audiovisual media and editorial workflows |
| M39 | planned | Robust tracking, calibration and reconstruction |
| M40 | planned | Complete shared editor workflows and device qualification |
| M41 | planned | Authored interchange and native format breadth |
| M42 | planned | Broader native Blender file semantics and edited export |
| M43 | planned | Discoverable API, SDKs and isolated extensions |
| M44 | planned | Authenticated collaboration and shared asset services |
| M45 | planned | Production asset pipeline, scale and operability |
| M46 | planned | Expanded suite qualification and measured comparison |
