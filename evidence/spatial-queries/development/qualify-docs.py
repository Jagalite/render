import json,sys
from pathlib import Path
root=Path(sys.argv[1]);e=Path('evidence/spatial-queries')
v=json.loads((root/'validation_report.json').read_text());c=json.loads((e/'collection.json').read_text());w=json.loads((e/'spatial-workflow/workflow_report.json').read_text());b=json.loads((e/'spatial-workflow/browser_report.json').read_text());cross=json.loads((e/'spatial-workflow/cross_platform.json').read_text())
assert all(r['status']=='passed' for r in [v,c,w,b,cross])
def replace(path,old,new):
 p=Path(path);s=p.read_text();assert old in s,(path,old);p.write_text(s.replace(old,new))
replace('docs/spatial_queries.md','# Mesh-local spatial query candidate','# Mesh-local spatial queries: mesh-local-spatial-v1')
replace('docs/spatial_queries.md','Qualification is pending. Sculpt displacement/mask chunks, affected-region','The bounded profile passed [native/browser qualification](../evidence/spatial-queries/README.md).\nSculpt displacement/mask chunks, affected-region')
replace('docs/project_cli.md','## Revision-pinned spatial query candidate','## Revision-pinned spatial queries')
replace('docs/api_and_agent_contracts.md','## Read-only spatial query candidate','## Read-only spatial queries')
replace('fixtures/spatial-queries/README.md','This\ncandidate is unqualified until complete evidence passes.','This\nbounded profile is qualified in `evidence/spatial-queries`.')
replace('fixtures/spatial-queries/README.md','Full preservation qualification remains pending.','Full preservation qualification passed; see `evidence/spatial-queries`.')
replace('docs/decision_records.md','**Candidate decision:** A typed asset-local sphere/nearest-surface query','**Decision:** A typed asset-local sphere/nearest-surface query')
p=Path('planning/milestones.json');data=json.loads(p.read_text());m=next(r for r in data['milestones'] if r['id']=='M10');assert m['status']=='in_progress';m['engine_subgates'].append(dict(id='spatial-queries',status='validated_experimental_profile',scope='read_only_mesh_query_prerequisite_for_sculpting_and_retopology',evidence='evidence/spatial-queries/README.md'));p.write_text(json.dumps(data,indent=2)+'\n')
p=Path('planning/feature_matrix.json');data=json.loads(p.read_text());assert not any(r['id']=='spatial_queries' for r in data['features']);data['features'].append(dict(id='spatial_queries',family='Mesh-local spatial queries',status='implemented_experimental_profile',first_useful_gate='M10',broader_gates=['M14'],qualification='mesh-local-spatial-v1: revision-pinned sphere and nearest-surface queries, stable decimal element IDs, bounded index construction and traversal, one-index cache and underflow-aware numerical metrics. Whole snapshot admission remains global; sculpting and retopology editing are separate.',capability_axes=dict(native_authoring='read_only_queries_consumed_by_ordinary_placement_transactions',native_evaluation='validated_mesh_asset_local_point_and_triangle_queries',final_render='validated_ordinary_placement_CPU_Metal_WebGPU_workflow',preview='shared_Rust_rendering; no_query_editor',human_editing='planned_M09',**{'import':'existing_retained_native_mesh_assets; no_implicit_scene_conversion','export':'typed_JSON_results_with_decimal_element_IDs; no_new_geometry_format'}),platform_profiles=dict(native_desktop='validated_macos_arm64; linux_windows_compile_only',browser_local='validated_Chromium_Rust_Wasm_OPFS_WebGPU; synchronous_bounded_queries'),evidence=['evidence/spatial-queries/README.md']));p.write_text(json.dumps(data,indent=2)+'\n')
p=Path('planning/engine-progress/README.md');s=p.read_text().replace('Current completed increment: bounded GPU geometry-buffer updates and resource observations; evidence/gpu-geometry-updates.','Current completed increment: revision-pinned mesh spatial queries and ordinary placement; evidence/spatial-queries.').replace('Next engine subgate: revision-pinned spatial queries and sparse sculpt displacement/mask chunks.','Next engine subgate: sparse sculpt displacement/mask chunks and affected-region geometry evaluation.').replace('UV and bounded tiled painting validated; sculpting, remesh and retopology pending','UV, bounded tiled painting and read-only spatial queries validated; sculpting, remesh and retopology editing pending');s+=f'''\nSpatial query prerequisite: {len(v['checks'])} acceptance checks, {v['native_test_count']} native\nand {v['wasm_test_count']} Wasm tests; {len(w['calls'])} CLI calls and {len(w['queries'])} query cases.\nNative/Wasm results, portable work counts, placement revisions and CPU passes agree\nexactly. High-ID archive/OPFS recovery, one-index eviction, aggregate polygon-work\nadmission and numerical underflow regressions pass. All {c['render_artifact_count']}\nprior render/pass/receipt artifacts and {c['prior_fixture_files']} fixtures are unchanged,\nwith no Cargo/dependency/shader changes. Whole snapshot validation remains global.\n''';p.write_text(s)
cold=w['queries'][0]['result']['cost'];web=b['queries'][0]['cost'];peaks=[r['maximum_resident_bytes'] for r in w['calls'] if 'maximum_resident_bytes' in r]
(e/'README.md').write_text(f'''# Spatial query acceptance

`mesh-local-spatial-v1` passed on baseline 4e8441c. It supplies read-only asset-local
sphere and nearest-surface queries with pinned revisions and stable decimal point,
face and corner IDs. Ordinary placement consumes query results through the existing
transaction API. This is an M10 prerequisite; sculpting, incremental evaluation,
multiresolution, remeshing and the deferred editor remain open.

The final run passed {len(v['checks'])} acceptance checks, {v['native_test_count']} native tests
and {v['wasm_test_count']} Wasm tests. Ten query tests cover plane/edge/vertex analytics,
scaled and tilted triangles, grid enumeration, nonmanifold/disconnected surfaces,
empty geometry, high IDs, exact ties, invalid/stale/budget rejection, read permissions,
exhaustive cancellation checkpoints and cache atomicity. Aggregate triangulation
charges and underflow boundaries down to 1e-300 have native/Wasm tests.

The reproducible fixture passes {len(w['calls'])} CLI calls and {len(w['queries'])} queries across a
291-point/512-triangle grid and a tiny triangle. Native and packaged browser results,
portable admission/traversal counts, placement revisions and CPU pixels/passes are
exact. The browser verifies one-index eviction and OPFS recovery while transporting
high-ID archives as opaque JSON strings. The old binary rejects the new operation
without changing state. Both demonstrated numerical bugs also pass through the wire.

The grid's first nearest query visits {cold['visited_nodes']} nodes and tests {cold['item_tests']}
triangles. Its source is {cold['source_bytes']} canonical bytes; the portable build charge
is {cold['build_byte_charge']} bytes and triangulation charge {cold['triangulation_work_charge']}.
Actual retained index layout is {cold['retained_index_layout_bytes']} bytes native and
{web['retained_index_layout_bytes']} bytes Wasm, excluding source allocation, Arc headers
and allocator overhead. Native one-shot query processes peak between {min(peaks)} and
{max(peaks)} resident bytes, including project loading and global snapshot validation.
Warm agents reuse the index, but document admission still validates/hashes every
retained asset. These are resource observations, not a comparative speedup claim.

Browser CPU/WebGPU RMSE is {cross['browser_cpu_gpu_rmse']}; Metal/WebGPU maximum channel
error is {cross['metal_webgpu_max_channel_error']}. Depth/normal and object-ID comparisons
pass their existing tolerances. Native archive recovery preserves CPU images exactly.
All {c['render_artifact_count']} prior render/pass/receipt artifacts (including GPU-fixture
cpu.pfm files) and {c['prior_fixture_files']} prior fixture files remain byte identical.
All five shaders, Cargo files, resolved packages/features and native links are
unchanged. No computational FFI, package or implicit external service was added.

Runtime qualification is macOS ARM64 Metal and Chrome WebGPU/OPFS; Linux and Windows
have compile checks. Exact native/Wasm package hashes, source and evidence manifests,
fixture reproduction and representation costs are retained. Development evidence
includes corrected test/fixture setup failures, deliberately interrupted runs for
review fixes, and original CLI probes demonstrating squared-area/distance underflow.
No rendering references were regenerated and no gate was weakened.

Reproduce with `scripts/validate-spatial-queries.py`, localhost:8783 and Chrome CDP
9224. Set the existing Blend reference/Blend/UV/painting baseline variables and
RENDER_SPATIAL_BASELINE_HOST to the protected 4e8441c host. Collect with
`scripts/collect-spatial-queries-evidence.py <run> <fresh-directory>` and set
RENDER_SPATIAL_PREVIOUS_RUN to the qualified GPU geometry-update run. Snapshot18,
persistent render receipts and ordinary mutation authority remain unchanged.
''')
print('Spatial query documentation qualified')
