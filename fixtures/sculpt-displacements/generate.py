"""Original CC0-1.0 triangular sculpt grid; no renderer-generated reference images."""
import json, sys
from pathlib import Path
side=8
positions=[[(x-side/2)/(side/2),(y-side/2)/(side/2),0.] for y in range(side+1) for x in range(side+1)]
faces=[]
for y in range(side):
 for x in range(side):
  a=y*(side+1)+x;faces.extend([[a,a+1,a+side+2],[a,a+side+2,a+side+1]])
edges=[];lookup={};corners=[];offsets=[0]
for face in faces:
 for a,b in zip(face,face[1:]+face[:1]):
  key=tuple(sorted((a,b)))
  if key not in lookup:lookup[key]=len(edges);edges.append(list(key))
  corners.append(dict(vertex=a,edge=lookup[key]))
 offsets.append(len(corners))
ids=[18446744073709551615-i for i in range(len(positions))]
mesh=dict(positions=dict(precision='F64',values=positions),edges=edges,face_offsets=offsets,corners=corners,point_ids=ids,edge_ids=list(range(1,len(edges)+1)),face_ids=list(range(1,len(faces)+1)),corner_ids=list(range(1,len(corners)+1)),attributes={})
value=dict(version=0,license='CC0-1.0',mesh=mesh,points=[dict(point=str(ids[40]),delta_meters=[0.,0.,0.375]),dict(point=str(ids[41]),delta_meters=[0.,0.,0.125])])
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
(out/'grid.json').write_text(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n')
