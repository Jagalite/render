"""Original CC0 varying HAIR controls, derived from the pinned original strand fixture."""
import hashlib,json,struct,sys
from pathlib import Path
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'data';root.mkdir(parents=True,exist_ok=True)
source=Path(__file__).resolve().parents[1]/'hair-import/data/arrays-le.hair';base=source.read_bytes();assert len(base)==356
colors=[[1,0,0],[0,1,0],[0,0,1],[.2,.4,.8],[.8,.2,.4],[.1,.8,.2],[1,.5,0]];transparency=[0,.1,.75,.25,.5,0,.9];rows=[]
for name in ['gradient-le','gradient-be','rgb-only','alpha-only','mixed-opacity']:
 b=bytearray(base);rgb=[[.8,.6,.2]]*7 if name=='alpha-only' else colors;t=[0]*7 if name=='rgb-only' else ([0]*3+transparency[3:] if name=='mixed-opacity' else transparency)
 b[244:272]=struct.pack('<7f',*t);b[272:356]=struct.pack('<21f',*(v for c in rgb for v in c));b[40:128]=b'Render original CC0 varying controls; metadata only'.ljust(88,b'\0')
 if name.endswith('-be'):
  for at in list(range(4,40,4))+list(range(132,356,4)):b[at:at+4]=b[at:at+4][::-1]
  for at in [128,130]:b[at:at+2]=b[at:at+2][::-1]
 p=root/(name+'.hair');p.write_bytes(b);rows.append({'file':p.name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'byte_order':'big_endian' if name.endswith('-be') else 'little_endian','rgb':rgb,'transparency':t})
(root/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','author':'Render project original varying controls','generator':'fixtures/curve-colors/generate.py','base_source':'fixtures/hair-import/data/arrays-le.hair','base_source_sha256':hashlib.sha256(base).hexdigest(),'files':rows},indent=2)+'\n')
