import json,math,random,subprocess
from pathlib import Path
random.seed(47)
for _ in range(100000):
 x,y=[random.uniform(1e6,1e8) for _ in range(2)];k=random.uniform(1.2,2.0)
 u,v=x*k,math.nextafter(y*k,-math.inf)
 original=x*v-y*u;s=max(x,y,u,v);normalized=(x/s)*(v/s)-(y/s)*(u/s)
 if original<0 and normalized==0:break
else:raise AssertionError('no case')
root=Path('artifacts/spatial-queries/development/conditioned-probe').resolve();root.mkdir()
def call(n,op):
 p=root/(n+'.json');p.write_text(json.dumps(dict(version=0,operation=op)));r=subprocess.run(['target/debug/render-host','project',str(root/'project'),str(p)],text=True,capture_output=True);(root/(n+'.stdout.json')).write_text(r.stdout);(root/(n+'.stderr.json')).write_text(r.stderr);assert r.returncode==0,r.stderr;return json.loads(r.stdout)
call('init',dict(method='init',document_id=f'{997:032x}'));state=call('empty',dict(method='inspect'))
mesh=dict(positions=dict(precision='F64',values=[[0.,0.,0.],[-y,x,0.],[x,y,0.],[u,v,0.]]),edges=[[0,1],[1,2],[2,3],[3,0]],face_offsets=[0,4],corners=[dict(vertex=i,edge=i) for i in range(4)],point_ids=[1,2,3,4],edge_ids=[1,2,3,4],corner_ids=[4,3,2,1],face_ids=[1],attributes={})
call('source',dict(method='apply',request=dict(version=0,base_revision=state['revision'],idempotency_key='spatial-conditioned-frame-probe',max_added_bytes=1<<20,commands=[dict(operation='put_mesh',mesh=mesh)])))
state=call('inspect',dict(method='inspect'))
q=dict(version=0,base_revision=state['revision'],mesh=next(iter(state['snapshot']['meshes'])),query=dict(kind='nearest_surface',point=[0,0,1],max_distance_meters=2),budget=dict(max_source_bytes=8<<20,max_build_bytes=32<<20,max_triangulation_work=16777216,max_visited_nodes=262144,max_item_tests=131072,max_hits=4096,max_result_bytes=1<<20))
r=call('query',dict(method='query_geometry',request=q));print(json.dumps(dict(original_area=original,normalized_area=normalized,coordinates=[x,y,u,v],hits=r['hits']),indent=2))
