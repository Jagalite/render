"""Original CC0 structural .blend containers; no Blender-generated implementation."""
import struct,sys,json,hashlib
from pathlib import Path
out=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).parent/'data';out.mkdir(parents=True,exist_ok=True)
manifest=[]
for width in [4,8]:
 for order in ['little','big']:
  prefix='<' if order=='little' else '>';pack=lambda fmt,*v:struct.pack(prefix+fmt,*v);ptr='I' if width==4 else 'Q';size=width+16
  def block(code,body,token=0,count=1):return pack('4sI'+ptr+'II',code,len(body),token,0,count)+body
  dna=bytearray(b'SDNANAME'+pack('I',3)+b'*next\0position[3]\0label[4]\0');dna.extend(bytes((-len(dna))%4));dna.extend(b'TYPE'+pack('I',3)+b'char\0float\0Node\0');dna.extend(bytes((-len(dna))%4));dna.extend(b'TLEN'+pack('3H',1,4,size));dna.extend(bytes((-len(dna))%4));dna.extend(b'STRC'+pack('I',1)+pack('8H',2,3,2,0,1,1,0,2))
  def node(next,xyz,label):return pack(ptr+'3f4s',next,*xyz,label)
  data=b'BLENDER'+(b'_' if width==4 else b'-')+(b'v' if order=='little' else b'V')+b'293'
  data+=block(b'REND',b'info',0x1000)
  data+=block(b'DATA',node(0x2000,[1.25,-2.5,3.75],b'one\0')+node(0,[4,5,6],b'two\0'),0x1000,2)
  data+=block(b'DATA',node(0,[7,8,9],b'end\0'),0x2000)
  data+=block(b'DNA1',dna)+block(b'ENDB',b'',count=0)
  name=f'{width*8}-{order}.blend';(out/name).write_bytes(data);manifest.append({'name':name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'pointer_bytes':width,'byte_order':order,'version':293})
(out/'provenance.json').write_text(json.dumps({'author':'Render project','license':'CC0-1.0','purpose':'Original minimal SDNA and relocation test corpus; not a Blender scene compatibility claim','files':manifest},indent=2)+'\n')
