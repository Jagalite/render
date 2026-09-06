"""Original sparse/dense variants of the repository's original animated ribbon.
Writes a NEW directory. No renderer or reviewed golden output is used.
"""
import copy, hashlib, json, struct, sys, zlib
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
original=Path(__file__).resolve().parent.parent/'animated-gltf'
source=json.loads((original/'character.gltf').read_text());source_bytes=(original/'character.bin').read_bytes()
axes={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16};width={5121:1,5123:2,5125:4,5126:4}
rows=[]
for a in source['accessors']:
 v=source['bufferViews'][a['bufferView']];offset=v.get('byteOffset',0)+a.get('byteOffset',0);size=width[a['componentType']]*axes[a['type']]
 rows.append([source_bytes[offset+i*size:offset+(i+1)*size] for i in range(a['count'])])
# Deliberately varied color quarters; PNG authored directly, losslessly.
def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00'+bytes([255,30,20,255,20,255,30,255])+b'\x00'+bytes([30,20,255,255,240,240,240,255])))+chunk(b'IEND',b'')
uv=[[0,0],[1,0],[1/3,1],[1,1]]
variants=['dense','sparse-zero-u16','sparse-base-u8']
for variant in variants:
 root=copy.deepcopy(source);root['asset']['generator']='Original sparse-accessor conformance fixture generator'
 binary=bytearray();views=[];accessors=[]
 def view(data,stride=None):
  while len(binary)%4:binary.append(0)
  v={'buffer':0,'byteOffset':len(binary),'byteLength':len(data)}
  if stride is not None:v['byteStride']=stride
  binary.extend(data);views.append(v);return len(views)-1
 all_rows=copy.deepcopy(rows);old_accessors=copy.deepcopy(source['accessors'])
 normal=len(old_accessors);old_accessors.append({'componentType':5126,'count':4,'type':'VEC3'});all_rows.append([struct.pack('<3f',0,0,1)]*4)
 uv_id=len(old_accessors);component=5126 if variant=='dense' else 5123 if variant.endswith('u16') else 5121
 old_accessors.append({'componentType':component,'count':4,'type':'VEC2',**({'normalized':True} if component!=5126 else {})})
 denominator=65535 if component==5123 else 255
 all_rows.append([struct.pack('<'+{5126:'f',5123:'H',5121:'B'}[component]*2,*([*p] if component==5126 else [round(v*denominator) for v in p])) for p in uv])
 for i,(old,data) in enumerate(zip(old_accessors,all_rows,strict=True)):
  a={k:v for k,v in old.items() if k not in ['bufferView','byteOffset']};size=len(data[0]);count=len(data)
  if variant=='dense':a['bufferView']=view(b''.join(data))
  else:
   # Packed normalized-u8 UV base keeps each vertex at four-byte alignment.
   # The u16 variant tests sparse UV overlays along with every other role.
   uv_byte=i==uv_id and component==5121
   if variant.startswith('sparse-base'):
    replaced=[count-1] if not uv_byte else []
    base=[b'\0'*size if j in replaced else row for j,row in enumerate(data)]
    interleaved=i in [0,normal,uv_id]
    stride=((size+3)//4)*4+4 if interleaved else size
    payload=b''.join(row+b'\xA5'*(stride-size) for row in base)[:(count-1)*stride+size]
    a['bufferView']=view(payload,stride if interleaved else None)
   else:replaced=[j for j,row in enumerate(data) if any(row)]
   if replaced:
    index_component=[5121,5123,5125][i%3];fmt={5121:'B',5123:'H',5125:'I'}[index_component]
    iv=view(struct.pack('<'+fmt*len(replaced),*replaced));vv=view(b''.join(data[j] for j in replaced))
    a['sparse']={'count':len(replaced),'indices':{'bufferView':iv,'componentType':index_component},'values':{'bufferView':vv}}
  accessors.append(a)
 primitive=root['meshes'][0]['primitives'][0];primitive['attributes'].update(NORMAL=normal,TEXCOORD_0=uv_id)
 root['materials'][0]['pbrMetallicRoughness'].update(baseColorFactor=[1,1,1,1],baseColorTexture={'index':0})
 root['images']=[{'bufferView':view(png),'mimeType':'image/png'}];root['samplers']=[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}];root['textures']=[{'source':0,'sampler':0}]
 root['bufferViews']=views;root['accessors']=accessors;root['buffers']=[{'uri':variant+'.bin','byteLength':len(binary)}]
 (out/(variant+'.gltf')).write_text(json.dumps(root,indent=2)+'\n');(out/(variant+'.bin')).write_bytes(binary)
 root['buffers'][0].pop('uri');jb=json.dumps(root,separators=(',',':')).encode();jb+=b' '*((-len(jb))%4);bb=bytes(binary);bb+=b'\0'*((-len(bb))%4)
 (out/(variant+'.glb')).write_bytes(struct.pack('<III',0x46546c67,2,28+len(jb)+len(bb))+struct.pack('<II',len(jb),0x4e4f534a)+jb+struct.pack('<II',len(bb),0x004e4942)+bb)
provenance={'license':'CC0-1.0','authorship':'Original repository synthetic character and checker texture; generated directly from glTF binary fields','inputs':[{'path':'fixtures/animated-gltf/'+p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [original/'character.gltf',original/'character.bin']],'artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir()) if p.is_file()]}
(out/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
