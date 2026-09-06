"""Original CC0 HAIR polylines; no cyHairFile runtime or external model data."""
import hashlib,json,struct,sys
from pathlib import Path
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'data';root.mkdir(parents=True,exist_ok=True)
rows=[]
def emit(name,order,optional=False):
 curves=[[(-.4,-.75,0),(-.35,0,.1),(-.25,.75,0)],[(.35,-.7,0),(.4,0,0),(.45,.7,0)]]
 if optional:curves[1]=[(.35,-.7,0),(.4,-.2,0),(.45,.2,0),(.4,.7,0)]
 pts=[p for c in curves for p in c];n=len(pts);flags=31 if optional else 2
 b=b'HAIR'+struct.pack(order+'4I5f',len(curves),n,flags,2,.12,0,.5,.25,.125)+b'Render original CC0 fixture; metadata only'.ljust(88,b'\0')
 assert len(b)==128
 if optional:b+=struct.pack(order+'2H',*(len(c)-1 for c in curves))
 b+=struct.pack(order+str(n*3)+'f',*(v for p in pts for v in p))
 if optional:
  b+=struct.pack(order+str(n)+'f',.12,.1,.08,.1,.12,.1,.06)
  b+=struct.pack(order+str(n)+'f',*([.25]*3+[0]*4))
  b+=struct.pack(order+str(n*3)+'f',*([.8,.2,.1]*3+[.1,.25,.8]*4))
 p=root/(name+'.hair');p.write_bytes(b);rows.append({'file':p.name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'byte_order':'little_endian' if order=='<' else 'big_endian','strands':len(curves),'points':n,'flags':flags})
for name,order,optional in [('default-le','<',False),('default-be','>',False),('arrays-le','<',True),('arrays-be','>',True)]:emit(name,order,optional)
(root/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','author':'Render project original analytic strands','generator':'fixtures/hair-import/generate.py','format_reference':'https://www.cemyuksel.com/research/hairmodels/','files':rows},indent=2)+'\n')
