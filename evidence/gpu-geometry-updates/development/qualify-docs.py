import json,sys
from pathlib import Path
root=Path(sys.argv[1]);e=Path('evidence/gpu-geometry-updates');validation=json.loads((root/'validation_report.json').read_text());collection=json.loads((e/'collection.json').read_text());assert validation['status']==collection['status']=='passed'
workflow=json.loads((e/'gpu-update-workflow/native_report.json').read_text());cross=json.loads((e/'gpu-update-workflow/cross_platform.json').read_text());resources=json.loads((e/'gpu_update_resources.json').read_text());assert workflow['status']==cross['status']==resources['status']=='passed'
def replace(path,old,new):
 p=Path(path);s=p.read_text();assert old in s,path;p.write_text(s.replace(old,new))
replace('docs/gpu_geometry_updates.md','# GPU geometry updates candidate','# GPU geometry updates: gpu-buffer-updates-v1')
replace('docs/gpu_geometry_updates.md','This renderer prerequisite for local sculpting is under validation.','This bounded renderer prerequisite passed its native/browser acceptance gates.')
replace('docs/gpu_geometry_updates.md','define the qualification gates. Native/browser GPU measurements and compatibility\nevidence are still pending.','define the qualification gates. Native/browser GPU measurements and compatibility\nevidence are retained in [the acceptance report](../evidence/gpu-geometry-updates/README.md).')
replace('fixtures/gpu-geometry-updates/README.md','This candidate is unqualified.','This bounded workflow is qualified in `evidence/gpu-geometry-updates`.')
replace('docs/decision_records.md','**Candidate decision:** Retain an exact byte shadow','**Decision:** Retain an exact byte shadow')
replace('planning/engine-progress/gpu-geometry-updates-contract.md','# GPU geometry update candidate and sculpt integration review','# GPU geometry update contract and sculpt integration review')
replace('planning/engine-progress/gpu-geometry-updates-contract.md','Candidate baseline: 24de830. No capability status is raised before native/browser\nworkflow and compatibility evidence pass.','Qualified baseline: 24de830. Native/browser workflow and compatibility evidence\npassed; see `evidence/gpu-geometry-updates`. This is a renderer prerequisite.')
p=Path('planning/feature_matrix.json');v=json.loads(p.read_text());r=next(x for x in v['features'] if x['id']=='render');r['qualification']+=' Packed geometry buffers retain allocations for same-size edits, patch bounded changed byte ranges, and report dense fallback and host shadow costs; gpu-buffer-updates-v1. Full host packing remains global.';r['evidence'].append('evidence/gpu-geometry-updates/README.md');p.write_text(json.dumps(v,indent=2)+'\n')
p=Path('planning/milestones.json');v=json.loads(p.read_text());m=next(x for x in v['milestones'] if x['id']=='M10');assert m['status']=='in_progress';m['engine_subgates'].append(dict(id='gpu-geometry-updates',status='validated_experimental_profile',scope='renderer_prerequisite_for_local_sculpting',evidence='evidence/gpu-geometry-updates/README.md'));p.write_text(json.dumps(v,indent=2)+'\n')
p=Path('planning/engine-progress/README.md');s=p.read_text();s=s.replace('Current completed increment: tiled texture painting, masks/layers, checkpoint recovery and explicit image baking; evidence/tiled-painting.','Current completed increment: bounded GPU geometry-buffer updates and resource observations; evidence/gpu-geometry-updates.');s=s.replace('Next engine subgate: localized sculpt authoring, spatial queries and sparse displacement/mask chunks.','Next engine subgate: revision-pinned spatial queries and sparse sculpt displacement/mask chunks.');s+=f'''\nGPU geometry update prerequisite: {len(validation['checks'])} acceptance checks,\n{validation['native_test_count']} native tests and {validation['wasm_test_count']} Wasm tests.\nThe {collection['native_cases']}-case actual GPU workflow demonstrates sparse patches,\nexact reuse, dense fallback, resize, cancellation and device recreation. Native\nand browser upload decisions/counts agree, and fresh/reused GPU outputs are exact.\nAll {collection['render_artifact_count']} prior render/pass/receipt files and\n{collection['prior_fixture_files']} prior fixtures remain unchanged. No new package,\nfeature or shader is introduced; the web adapter adds a direct Wasm serde edge.\nM10 sculpting still requires localized spatial queries, sparse chunks and affected\nregion evaluation; current host packing remains global.\n''';p.write_text(s)
local=next(x for x in workflow['cases'] if x['name']=='local')['statistics']['last'];dense=next(x for x in workflow['cases'] if x['name']=='dense')['statistics']['last']
(e/'README.md').write_text(f'''# GPU geometry update acceptance

The bounded `gpu-buffer-updates-v1` renderer prerequisite passed on painting baseline
24de830. It reuses exact buffers, patches changed aligned records, rewrites dense or
fragmented same-size inputs in place and reallocates changed sizes. M10 sculpting
and M09 remain open; full host evaluation/packing is still global.

The final run passed {len(validation['checks'])} acceptance checks,
{validation['native_test_count']} native tests and {validation['wasm_test_count']} Wasm tests.
The {collection['native_cases']}-case actual GPU workflow authors mesh bindings through ordinary
procedural/document transactions and compares every result with a fresh renderer.
It covers local edits, retained-snapshot undo, dense edits, resize, camera/material
changes, cancellation, invalid/stale inputs and device recreation.

A one-point edit uploads {local['uploaded_bytes']} bytes in {local['queue_write_calls']} queue writes out of
{local['packed_buffer_bytes']} packed bytes, without reallocating. Repeating the same input writes
zero geometry bytes. A broad edit triggers a {dense['uploaded_bytes']}-byte rewrite in the
same allocation. Native and browser report identical decisions and byte counts;
native/Wasm CPU pixels are exact, and fresh/reused GPU pixels, passes and receipts
are exact within each backend. Cross-backend GPU comparison uses existing numerical
tolerances; see `gpu-update-workflow/cross_platform.json`.

`gpu_update_resources.json` measures the precompiled conformance process separately
from Cargo: {resources['seconds']:.6g} seconds and {resources['maximum_resident_set_bytes']} peak resident bytes on this
shared macOS host. This includes fixture transactions, CPU renders, multiple GPU
adapters and serialization. It is not a comparative benchmark or allocator cap.
`representation_costs.json` distinguishes packed host records, serialization, the
retained shadow and transferred GPU bytes; driver/staging and other forms are extra.

All {collection['render_artifact_count']} prior render/pass/receipt files and
{collection['prior_fixture_files']} prior fixture files remain byte identical. All five shaders and
resolved native/browser packages, features and native links are unchanged. Cargo
adds only the web Wasm target's direct edge to already-pinned serde 1.0.228 with
existing derive/rc features. No new computational FFI or package is added.
The new grid input reproduces byte for byte, including its independent one-point,
dense and topology-resize variants. The measured resource replay reproduces the
main workflow's documents and render artifacts exactly.

Runtime evidence is macOS ARM64 Metal and Chrome WebGPU; Linux/Windows receive
compile checks. The exact served Wasm hash and actual native GPU test binary hash
are retained. `development/` preserves the initial missing-serde browser compile
failure and the first test output relocation caused by Cargo's package working
directory. These were corrected without weakening gates or regenerating goldens.

Reproduce with `scripts/validate-gpu-geometry-updates.py`, serving the candidate
package on localhost:8782 with Chrome CDP on 9224. Supply the retained Blender
reference and old Blend/UV/painting baseline binaries through the environment names
used by the validator. Collect with `scripts/collect-gpu-geometry-updates-evidence.py
<run> <fresh-directory>` and set `RENDER_GPU_UPDATES_PREVIOUS_RUN` to the qualified
painting run. Existing rendering receipts and snapshot schemas are unchanged.
''')
print('GPU geometry update documentation qualified')
