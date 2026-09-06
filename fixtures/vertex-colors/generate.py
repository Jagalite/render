"""Original CC0 linear vertex colors. New output directory; no renderer input."""
import hashlib,json,struct,sys,zlib
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
for variant in ['rgb','rgba','u8','u16','sparse']:
 binary=bytearray();views=[];accessors=[]
 def view(data,stride=None):
  binary.extend(b'\0'*(-len(binary)%4));v={'buffer':0,'byteOffset':len(binary),'byteLength':len(data)}
  if stride:v['byteStride']=stride
  views.append(v);binary.extend(data);return len(views)-1
 def accessor(rows,kind,component,fmt,normalized=False):
  a={'bufferView':view(b''.join(struct.pack('<'+fmt,*r) for r in rows)),'componentType':component,'type':kind,'count':len(rows)}
  if normalized:a['normalized']=True
  accessors.append(a);return len(accessors)-1
 def chunk(k,d):return struct.pack('>I',len(d))+k+d+struct.pack('>I',zlib.crc32(k+d))
 pos=accessor([[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],'VEC3',5126,'3f');accessors[pos].update(min=[-1,-1,0],max=[1,1,0])
 normal=accessor([[0,0,1]]*4,'VEC3',5126,'3f')
 uv=accessor([[0,0],[1,0],[1,1],[0,1]],'VEC2',5126,'2f')
 idx=accessor([[0],[1],[2],[0],[2],[3]],'SCALAR',5123,'H')
 colors=[[0,0,1,0],[1,0,0,1],[1,1,0,1],[0,1,1,0]]
 if variant=='rgb':color=accessor([c[:3] for c in colors],'VEC3',5126,'3f')
 elif variant=='sparse':
  indices=view(bytes(range(4)));values=view(b''.join(struct.pack('<4f',*c) for c in colors))
  accessors.append({'componentType':5126,'type':'VEC4','count':4,'sparse':{'count':4,'indices':{'bufferView':indices,'componentType':5121},'values':{'bufferView':values}}});color=len(accessors)-1
 elif variant in ['u8','u16']:
  limit,component,fmt=(255,5121,'4B') if variant=='u8' else (65535,5123,'4H')
  color=accessor([[x*limit for x in c] for c in colors],'VEC4',component,fmt,True)
 else:color=accessor(colors,'VEC4',5126,'4f')
 png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+bytes([128,192,255,128])))+chunk(b'IEND',b'')
 image=view(png)
 attrs={'POSITION':pos,'NORMAL':normal,'TEXCOORD_0':uv}
 root={'asset':{'version':'2.0','generator':'Original linear vertex color analytic fixture'},'scene':0,'scenes':[{'nodes':[0,1]}],
  'nodes':[{'name':'colored','mesh':0,'translation':[-1.25,0,0]},{'name':'uncolored-shared-material','mesh':1,'translation':[1.25,0,0]}],
  'meshes':[{'primitives':[{'attributes':dict(attrs,COLOR_0=color),'indices':idx,'material':0}]},{'primitives':[{'attributes':attrs,'indices':idx,'material':0}]}],
  'materials':[{'name':'shared-linear-color','doubleSided':True,'emissiveFactor':[.1,.2,.3],
   'pbrMetallicRoughness':{'baseColorFactor':[.6,.4,.2,.75],'metallicFactor':0,'roughnessFactor':1,'baseColorTexture':{'index':0}}}],
  'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],
  'textures':[{'source':0,'sampler':0}],'images':[{'bufferView':image,'mimeType':'image/png'}],
  'bufferViews':views,'accessors':accessors,'buffers':[{'uri':variant+'.bin','byteLength':len(binary)}]}
 (out/(variant+'.gltf')).write_text(json.dumps(root,indent=2)+'\n');(out/(variant+'.bin')).write_bytes(binary)
 del root['buffers'][0]['uri'];j=json.dumps(root,separators=(',',':')).encode();j+=b' '*(-len(j)%4);b=bytes(binary)+b'\0'*(-len(binary)%4)
 (out/(variant+'.glb')).write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
(out/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','authorship':'Original two quads, linear RGB/A arrays and directly encoded PNG', 'oracle':'Colored local u=(x+1)/2,v=(y+1)/2: RGB=(u,v,1-u), A=u except RGB A=1. Uncolored primitive shares source material and defaults to white. Base factor=(.6,.4,.2), sRGB texture bytes=(128,192,255), factor alpha=.75, texture alpha=128/255. Emission=(.1,.2,.3) independent.', 'artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir())]},indent=2)+'\n')
