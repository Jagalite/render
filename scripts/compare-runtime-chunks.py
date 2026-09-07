"""Compare observed copy work in the identical native and Wasm Rust fixture."""
import json,re,sys
from pathlib import Path
root=Path(sys.argv[1]);rows={}
for platform in ['native','wasm']:
 text=(root/'logs'/('chunks_'+platform+'.txt')).read_text()
 found=re.findall(r'CHUNK_RESOURCE (\{[^\n]+\})',text)
 assert len(found)==1,(platform,found)
 assert re.search(r'test result: ok\. 5 passed',text),platform
 rows[platform]=json.loads(found[0])
portable=[k for k in rows['native'] if 'bytes' not in k]
assert {k:rows['native'][k] for k in portable}=={k:rows['wasm'][k] for k in portable}
assert rows['native']['triangle_chunks_copied']==1
assert rows['native']['triangle_elements_cloned']==64
assert rows['native']['triangle_root_references_copied']==64
report={'status':'passed','scope':'4096 generated triangles, 2047 BVH nodes; copy/storage prerequisite only, no refit or sculpt operation; full admission and host GPU packing remain global','portable_counts_equal':True,'observations':rows}
(root/'chunk_resources.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
