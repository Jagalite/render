"""Original CC0-1.0 high-ID mesh and query inputs; no generated image references."""
import json,sys
from pathlib import Path
positions=[[(x-8)/4,(y-8)/4,0.] for y in range(17) for x in range(17)]
faces=[]
for y in range(16):
 for x in range(16):
  a=y*17+x;faces.extend([[a,a+1,a+18],[a,a+18,a+17]])
edges=[];lookup={};corners=[];offsets=[0]
for face in faces:
 for a,b in zip(face,face[1:]+face[:1]):
  key=tuple(sorted((a,b)))
  if key not in lookup:lookup[key]=len(edges);edges.append(list(key))
  corners.append(dict(vertex=a,edge=lookup[key]))
 offsets.append(len(corners))
# Loose coincident points exercise distinct durable identity and closed spheres.
positions.extend([[0.,0.,0.],[0.,0.,0.]])
mesh=dict(positions=dict(precision='F64',values=positions),edges=edges,face_offsets=offsets,corners=corners,
 point_ids=[18446744073709551615-i for i in range(len(positions))],edge_ids=list(range(1,len(edges)+1)),
 face_ids=[9007199254741993-i for i in range(len(faces))],corner_ids=[9007199254750993-i for i in range(len(corners))],attributes={})
value=dict(version=0,license='CC0-1.0',mesh=mesh,queries=[
 dict(kind='nearest_surface',point=[0.625,0.375,1.],max_distance_meters=1.),
 dict(kind='points_in_sphere',center=[0.,0.,0.],radius_meters=0.),
 dict(kind='points_in_sphere',center=[0.5,0.,0.],radius_meters=0.5),
 dict(kind='nearest_surface',point=[3.,3.,1.],max_distance_meters=2.),
 dict(kind='nearest_surface',point=[0.,0.,1.],max_distance_meters=0.5)])
value['tiny']=dict(positions=dict(precision='F64',values=[[0.,0.,0.],[1.4e-81,0.,0.],[0.,1.4e-81,0.]]),edges=[[0,1],[1,2],[2,0]],face_offsets=[0,3],corners=[dict(vertex=i,edge=i) for i in range(3)],point_ids=[1,2,3],edge_ids=[1,2,3],corner_ids=[1,2,3],face_ids=[1],attributes={})
value['tiny_queries']=[dict(kind='nearest_surface',point=[3.5e-82,3.5e-82,1e-81],max_distance_meters=1e-80),dict(kind='points_in_sphere',center=[0,0,1e-200],radius_meters=0)]
x,y,u,v=4293147.105974088,38042555.927906096,7513355.533717836,66577557.4519595
value['conditioned']=dict(positions=dict(precision='F64',values=[[0.,0.,0.],[-y,x,0.],[x,y,0.],[u,v,0.]]),edges=[[0,1],[1,2],[2,3],[3,0]],face_offsets=[0,4],corners=[dict(vertex=i,edge=i) for i in range(4)],point_ids=[1,2,3,4],edge_ids=[1,2,3,4],corner_ids=[4,3,2,1],face_ids=[1],attributes={})
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
(out/'high-id-grid.json').write_text(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n')
