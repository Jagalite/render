"""Original CC0 grid inputs for a persistent GPU cache workflow; no image oracle."""
import copy,json,sys
from pathlib import Path

def mesh(positions,faces):
 edges=[];lookup={};corners=[];offsets=[0]
 for face in faces:
  for a,b in zip(face,face[1:]+face[:1]):
   key=tuple(sorted((a,b)))
   if key not in lookup:lookup[key]=len(edges);edges.append(list(key))
   corners.append(dict(vertex=a,edge=lookup[key]))
  offsets.append(len(corners))
 return dict(positions=dict(precision='F32',values=positions),edges=edges,face_offsets=offsets,corners=corners,point_ids=list(range(1,len(positions)+1)),edge_ids=list(range(1,len(edges)+1)),face_ids=list(range(1,len(faces)+1)),corner_ids=list(range(1,len(corners)+1)),attributes={})
side=17
positions=[[(x-8)/4,(y-8)/4,0] for y in range(side) for x in range(side)]
faces=[]
for y in range(side-1):
 for x in range(side-1):
  a=y*side+x;b=a+1;c=a+side+1;d=a+side
  faces.extend([[a,b,c],[a,c,d]])
base=mesh(positions,faces);local=copy.deepcopy(base)
local['positions']['values'][8*side+8][2]=0.125
resized=mesh(positions+[[3,0,0],[3.25,0,0],[3,0.25,0]],faces+[[len(positions),len(positions)+1,len(positions)+2]])
value=dict(version=0,source_license='CC0-1.0',grid_side=side,local_point_id=8*side+9,local_height_meters=0.125,base=base,local=local,resized=resized)
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
(out/'grid-local-edit.json').write_text(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n')
