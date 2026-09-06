"""Original CC0 multiple UV fixture. New output directory; no renderer input."""
import hashlib,json,struct,sys,zlib
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
binary=bytearray();views=[];accessors=[]
def view(data):
 binary.extend(b'\0'*(-len(binary)%4));views.append({'buffer':0,'byteOffset':len(binary),'byteLength':len(data)});binary.extend(data);return len(views)-1
def accessor(rows,kind,component,fmt,normalized=False):
 a={'bufferView':view(b''.join(struct.pack('<'+fmt,*r) for r in rows)),'componentType':component,'type':kind,'count':len(rows)}
 if normalized:a['normalized']=True
 accessors.append(a);return len(accessors)-1
def chunk(k,d):return struct.pack('>I',len(d))+k+d+struct.pack('>I',zlib.crc32(k+d))
def png(pixels):
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+bytes(pixels[:8])+b'\0'+bytes(pixels[8:])))+chunk(b'IEND',b'')
pos=accessor([[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],'VEC3',5126,'3f');accessors[pos].update(min=[-1,-1,0],max=[1,1,0])
normal=accessor([[0,0,1]]*4,'VEC3',5126,'3f')
uv0=accessor([[0,0],[1,0],[1,1],[0,1]],'VEC2',5126,'2f')
uv1=accessor([[0,0],[0,65535],[65535,65535],[65535,0]],'VEC2',5123,'2H',True)
# Normalized byte VEC2 vertices use four-byte stride/alignment.
data=b''.join(bytes([x,y,0,0]) for x,y in [[255,0],[0,0],[0,255],[255,255]])
v=view(data);views[v]['byteStride']=4;accessors.append({'bufferView':v,'componentType':5121,'count':4,'type':'VEC2','normalized':True});uv2=len(accessors)-1
uv7=accessor([[1/3,2/3]]*4,'VEC2',5126,'2f')
idx=accessor([[0],[1],[2],[0],[2],[3]],'SCALAR',5123,'H')
images=[]
for pixels in [[255,30,20,0,20,255,30,85,30,20,255,170,240,240,240,255],[204,128,230,255]*4,[64,128,192,255,128,192,64,255,192,64,128,255,255,255,255,255]]:
 images.append({'bufferView':view(png(pixels)),'mimeType':'image/png'})
root={'asset':{'version':'2.0','generator':'Original named UV analytic fixture'},'scene':0,'scenes':[{'nodes':[0]}],
 'nodes':[{'name':'multi-uv-quad','mesh':0}],
 'meshes':[{'primitives':[{'attributes':{'POSITION':pos,'NORMAL':normal,'TEXCOORD_0':uv0,'TEXCOORD_1':uv1,'TEXCOORD_2':uv2,**{f'TEXCOORD_{n}':uv7 for n in range(3,8)}},'indices':idx,'material':0}]}],
 'materials':[{'name':'independent-role-UVs','doubleSided':True,'emissiveFactor':[.4,.4,.4],
  'pbrMetallicRoughness':{'baseColorFactor':[.6,.6,.6,1],'metallicFactor':.5,'roughnessFactor':.8,'baseColorTexture':{'index':0},'metallicRoughnessTexture':{'index':2,'texCoord':7}},
  'emissiveTexture':{'index':0,'texCoord':1},'occlusionTexture':{'index':2,'texCoord':2},'normalTexture':{'index':1,'texCoord':1}}],
 'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],
 'textures':[{'source':i,'sampler':0} for i in range(3)],'images':images,'bufferViews':views,'accessors':accessors,'buffers':[{'uri':'roles.bin','byteLength':len(binary)}]}
(out/'roles.gltf').write_text(json.dumps(root,indent=2)+'\n');(out/'roles.bin').write_bytes(binary)
del root['buffers'][0]['uri'];j=json.dumps(root,separators=(',',':')).encode();j+=b' '*(-len(j)%4);b=bytes(binary)+b'\0'*(-len(binary)%4)
(out/'roles.glb').write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
(out/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','authorship':'Original quad, UV arrays and directly encoded PNGs','oracle':'UV0=(x,y), UV1=(y,x), UV2=(1-x,y), UV3 through UV7=(1/3,2/3). Tangent fallback for UV1 points +Y with negative handedness.', 'artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir())]},indent=2)+'\n')
