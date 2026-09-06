"""Original CC0 VOL3 fixtures. Standard-library-only, no foreign volume runtime."""
import hashlib,json,struct,sys
from pathlib import Path
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'data';root.mkdir(parents=True,exist_ok=True)
def grid(name,dims,channels,values,bounds=(0,0,0,2,2,2)):
    assert len(values)==dims[0]*dims[1]*dims[2]*channels
    data=b'VOL\x03'+struct.pack('<5i6f',1,*dims,channels,*bounds)+struct.pack('<'+'f'*len(values),*values)
    (root/(name+'.vol')).write_bytes(data)
    return {'file':name+'.vol','dimensions':dims,'channels':channels,'bounds':bounds,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}
rows=[]
rows.append(grid('homogeneous',(2,2,2),1,[1]*8))
rows.append(grid('emission',(2,2,2),3,[.25,.5,1]*8))
rows.append(grid('axis',(2,2,2),1,[1,2,3,4,5,6,7,8]))
rows.append(grid('empty',(2,2,2),1,[0]*8))
v=[0]*(64**3)
for x,y,z,d in [(0,0,0,1),(63,0,0,2),(0,63,0,3),(0,0,63,4),(63,63,63,5)]:v[(z*64+y)*64+x]=d
rows.append(grid('sparse',(64,64,64),1,v,bounds=(-1,-1,-1,1,1,1)))
(root/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','author':'Render project original analytic fixtures','generator':'fixtures/volume-import/generate.py','layout':'VOL3 little-endian float32; x fastest, then y, then z; interleaved channels','files':rows},indent=2)+'\n')
