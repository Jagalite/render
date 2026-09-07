import json,subprocess
from pathlib import Path
root=Path('artifacts/spatial-queries/development/tiny-probe').resolve();root.mkdir()
def call(n,op):
 p=root/(n+'.json');p.write_text(json.dumps(dict(version=0,operation=op)))
 r=subprocess.run(['target/debug/render-host','project',str(root/'project'),str(p)],text=True,capture_output=True)
 (root/(n+'.stdout.json')).write_text(r.stdout);(root/(n+'.stderr.json')).write_text(r.stderr);assert r.returncode==0,r.stderr
 return json.loads(r.stdout)
call('init',dict(method='init',document_id=f'{998:032x}'));s=call('empty',dict(method='inspect'))
mesh=dict(positions=dict(precision='F64',values=[[0,0,0],[1.4e-81,0,0],[0,1.4e-81,0]]),edges=[[0,1],[1,2],[2,0]],face_offsets=[0,3],corners=[dict(vertex=i,edge=i) for i in range(3)],point_ids=[1,2,3],edge_ids=[1,2,3],corner_ids=[1,2,3],face_ids=[1],attributes={})
call('source',dict(method='apply',request=dict(version=0,base_revision=s['revision'],idempotency_key='spatial-tiny-triangle-probe',max_added_bytes=1<<20,commands=[dict(operation='put_mesh',mesh=mesh)])))
s=call('inspect',dict(method='inspect'))
q=dict(version=0,base_revision=s['revision'],mesh=next(iter(s['snapshot']['meshes'])),query=dict(kind='nearest_surface',point=[3.5e-82,3.5e-82,1e-81],max_distance_meters=1e-80),budget=dict(max_source_bytes=8<<20,max_build_bytes=32<<20,max_triangulation_work=16777216,max_visited_nodes=262144,max_item_tests=131072,max_hits=4096,max_result_bytes=1<<20))
r=call('query',dict(method='query_geometry',request=q));print(json.dumps(r['hits'],indent=2))
