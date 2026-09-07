import json,sys
from pathlib import Path
root=Path(sys.argv[1]);e=Path('evidence/runtime-geometry-chunks')
v=json.loads((root/'validation_report.json').read_text());c=json.loads((e/'collection.json').read_text());r=json.loads((e/'chunk_resources.json').read_text());probe=json.loads((e/'chunk_probe_comparison.json').read_text());delta=json.loads((e/'spatial_layout_delta.json').read_text())
assert all(x['status']=='passed' for x in [v,c,r,probe,delta])
def replace(path,old,new):
 p=Path(path);s=p.read_text();assert old in s,(path,old);p.write_text(s.replace(old,new))
replace('docs/runtime_geometry_chunks.md','Candidate qualification is pending.','The bounded storage prerequisite passed [qualification](../evidence/runtime-geometry-chunks/README.md).')
replace('fixtures/runtime-geometry-chunks/README.md','Qualification is pending.','Qualification passed; see `evidence/runtime-geometry-chunks/README.md`.')
replace('docs/decision_records.md','**Candidate decision:** Keep evaluated triangle and BVH node order','**Decision:** Keep evaluated triangle and BVH node order')
replace('planning/engine-progress/runtime-geometry-chunks-review.md','# Runtime geometry chunk integration review (candidate)','# Runtime geometry chunk integration review')
replace('planning/engine-progress/runtime-geometry-chunks-review.md','Final qualification is pending.',f"Final qualification passed {len(v['checks'])} checks, {v['native_test_count']} native tests and {v['wasm_test_count']} Wasm tests. The collector also compared 26 sampler probe artifacts with the qualified pre-change binary. See evidence/runtime-geometry-chunks for package hashes, observations and unchanged outputs.")
p=Path('planning/milestones.json');d=json.loads(p.read_text());m=next(x for x in d['milestones'] if x['id']=='M10');assert m['status']=='in_progress';assert not any(x['id']=='runtime-geometry-chunks' for x in m['engine_subgates']);m['engine_subgates'].append(dict(id='runtime-geometry-chunks',status='validated_experimental_profile',scope='immutable_evaluated_storage_prerequisite; no_refit_or_sculpt_claim',evidence='evidence/runtime-geometry-chunks/README.md'));p.write_text(json.dumps(d,indent=2)+'\n')
replace('planning/engine-progress/README.md','Current completed increment: revision-pinned mesh spatial queries and ordinary placement; evidence/spatial-queries.','Current completed increment: immutable evaluated geometry chunks; evidence/runtime-geometry-chunks.')
replace('planning/engine-progress/README.md','Next engine subgate: sparse sculpt displacement/mask chunks and affected-region geometry evaluation.','Next engine subgate: bounded BVH refit correspondence, then sparse sculpt displacement/mask authoring and affected-region evaluation.')
p=Path('planning/engine-progress/README.md');s=p.read_text()+f'''\nRuntime geometry storage prerequisite: {len(v['checks'])} checks, {v['native_test_count']} native\nand {v['wasm_test_count']} Wasm tests. A one-triangle replacement in 4,096 triangles\ncopies 64 payloads and 64 root references, with 3,072 bytes of nested UV data.\nNative/Wasm portable counts agree. All {c['render_artifact_count']} earlier rendered\nartifacts and {c['prior_fixture_files']} fixtures remain exact; an additional 26\nsampler probe artifacts match the qualified old binary. Query answers and portable\nbudgets are unchanged; retained layout observations reflect chunks. No dependency\nor shader change. Refitting, sculpting, global admission and host packing remain open.\n''';p.write_text(s)
p=Path('docs/spatial_queries.md');p.write_text(p.read_text()+'''\nThe renderer's later immutable runtime chunk representation preserves this query\nprofile's answers and portable costs. Updated retained-layout observations and\nbyte-identical baseline comparisons are in `evidence/runtime-geometry-chunks`.\n''')
n=r['observations']['native'];w=r['observations']['wasm']
(e/'README.md').write_text(f'''# Immutable runtime geometry acceptance

Evaluated triangle and BVH node storage now shares immutable 64-item chunks. This
bounded prerequisite passed on baseline c5e7233; M10 sculpting and refits remain open.
No document version, durable schema, operation, renderer arithmetic or shader changed.

Qualification passed {len(v['checks'])} checks, {v['native_test_count']} native tests and
{v['wasm_test_count']} Wasm tests. Five storage tests cover boundary sizes, forward/reverse
iteration, actual Clone counts, old-root immutability, invalid slots, every cancellation
checkpoint, nested allocations and an independent diffuse-plane render. All existing
CLI, packaged Chrome WebGPU/OPFS and Metal workflows pass. Linux/Windows are compile
checks; this does not claim their runtime qualification.

In the 4,096-triangle/2,047-node fixture, one triangle replacement clones 64 triangles,
one payload chunk and 64 root references. The 63 unaffected triangle chunks remain
shared. Nested UV clones add 3,072 bytes. A no-op final-node replacement clones 63 node
payloads and 32 root references; its leaf-item clones add {n['node_nested_bytes_cloned']}
bytes native and {w['node_nested_bytes_cloned']} bytes Wasm. It is not a refit test.

| Container layout excluding nested allocations | Native bytes | Wasm bytes |
|---|---:|---:|
| Triangles | {n['triangle_retained_layout_bytes']} | {w['triangle_retained_layout_bytes']} |
| BVH nodes | {n['node_retained_layout_bytes']} | {w['node_retained_layout_bytes']} |

These layout counts exclude Self, Arc headers and allocator overhead. Initial conversion
retains the flat builder allocation while moving into chunks. Root table replacement
remains global, as do changed-geometry evaluation, snapshot admission and GPU host
packing. Whole-process/time observations in chunk_probe_comparison.json are sequential,
uncontrolled observations including GPU initialization; no speedup claim follows.

The preservation collector checked {c['render_artifact_count']} prior render/pass/receipt
artifacts and {c['prior_fixture_files']} prior fixture files byte for byte. Query results
and portable work costs match the old binary, with layout changes separately recorded.
The adapted PBR sampler probe additionally matches 26 old-binary image/pass/receipt
and analytic report files. Exact old/new tested packages and source manifests are
recorded; dependencies, enabled features, native links and all five shaders are unchanged.

Development logs preserve the missing post-clean cache directory, missing first accessor,
old mutable diagnostic setups, and the failed Wasm observation collection before use of
the runner's console macro. No assertion or image tolerance was relaxed. The pinned
Khronos validator was restored after temporary files disappeared and its three file
hashes matched prior qualification; Blender reference bytes came from the previous
qualified exact-source export. Neither tool is a runtime dependency.

Reproduce with scripts/validate-runtime-geometry-chunks.py using localhost:8784 and
Chrome CDP 9224 plus the prior Blend/UV/paint/spatial baseline environment variables.
Collect with scripts/collect-runtime-geometry-chunks-evidence.py and
RENDER_CHUNKS_PREVIOUS_RUN set to the qualified spatial-query run. Preserve the c5e7233
baseline binary/provenance under artifacts/runtime-geometry-chunks/baseline for the
additional sampler comparison. Source schemas and authored transaction authority stay
unchanged; the storage replacement seam only accepts private runtime slots.
''')
print('Qualified runtime geometry documentation.')
