"""Compare measured native/Wasm resource counters, never platform layouts as equal."""
import json,re,sys
from pathlib import Path
native,wasm,output=map(Path,sys.argv[1:])
def load(p):
 rows=[json.loads(line.split('SCULPT_RESOURCE ',1)[1]) for line in p.read_text().splitlines() if 'SCULPT_RESOURCE {' in line]
 assert len(rows)==1,(p,len(rows));return rows[0]
a,b=load(native),load(wasm)
layout=['basis_geometry_layout_bytes','basis_nested_layout_bytes','basis_mapping_layout_bytes','current_geometry_layout_bytes','current_nested_layout_bytes']
comparisons=[]
for phase in ['cold','warm']:
 x,y=dict(a[phase]),dict(b[phase]);actual={k:dict(native=x.pop(k),wasm=y.pop(k)) for k in layout};assert x==y,(phase,x,y)
 comparisons.append(dict(phase=phase,portable=x,layouts=actual))
assert a['base_current_shared_triangle_chunks']==b['base_current_shared_triangle_chunks']
assert a['base_current_shared_node_chunks']==b['base_current_shared_node_chunks']
result=dict(status='passed',observations=comparisons,shared_triangle_chunks=a['base_current_shared_triangle_chunks'],shared_node_chunks=a['base_current_shared_node_chunks'],scope=a['scope'])
output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
