"""Original CC0 secondary-only checker emitter; no renderer or reference images."""
import copy,hashlib,json,struct,sys,zlib
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
binary=bytearray();views=[];accessors=[]
def view(data):
    binary.extend(b'\0'*(-len(binary)%4));views.append({'buffer':0,'byteOffset':len(binary),'byteLength':len(data)});binary.extend(data);return len(views)-1
def accessor(rows,kind,component,fmt):
    accessors.append({'bufferView':view(b''.join(struct.pack('<'+fmt,*r) for r in rows)),'componentType':component,'type':kind,'count':len(rows)});return len(accessors)-1
def chunk(k,d):return struct.pack('>I',len(d))+k+d+struct.pack('>I',zlib.crc32(k+d))
scan=b''.join(b'\0'+b''.join(bytes([255*((x+y)%2)]*3+[255]) for x in range(32)) for y in range(32))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,32,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(scan))+chunk(b'IEND',b'')
(out/'checker.png').write_bytes(png)
pos=accessor([[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],'VEC3',5126,'3f');accessors[pos].update(min=[-1,-1,0],max=[1,1,0])
normal=accessor([[0,0,1]]*4,'VEC3',5126,'3f');uv=accessor([[0,0],[1,0],[1,1],[0,1]],'VEC2',5126,'2f');idx=accessor([[0],[1],[2],[0],[2],[3]],'SCALAR',5123,'H');image=view(png)
primitive={'attributes':{'POSITION':pos,'NORMAL':normal,'TEXCOORD_0':uv},'indices':idx}
root={'asset':{'version':'2.0','generator':'Original secondary texture footprint fixture'},'scene':0,'scenes':[{'nodes':[0,1]}],
'nodes':[{'name':'untextured-floor','mesh':0},{'name':'checker-ceiling','mesh':1,'translation':[0,0,2],'scale':[4,4,1]}],
'meshes':[{'primitives':[dict(primitive,material=i)]} for i in range(2)],
'materials':[{'name':'floor','doubleSided':True,'pbrMetallicRoughness':{'baseColorFactor':[.5,.5,.5,1],'metallicFactor':0,'roughnessFactor':.5}},
{'name':'emitter','doubleSided':True,'pbrMetallicRoughness':{'baseColorFactor':[0,0,0,1],'metallicFactor':0,'roughnessFactor':1},'emissiveFactor':[1,1,1],'emissiveTexture':{'index':0}}],
'images':[{'bufferView':image,'mimeType':'image/png'}],'textures':[{'source':0,'sampler':0}],
'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],'accessors':accessors,'bufferViews':views,'buffers':[{'byteLength':len(binary)}]}
for profile in ['opaque','mask']:
    for filtering in ['nearest','mip']:
        name=profile+'-'+filtering;r=copy.deepcopy(root)
        r['materials'][0]['alphaMode']=profile.upper()
        if filtering=='mip':r['samplers'][0]['minFilter']=9987
        r['buffers'][0]['uri']=name+'.bin';(out/(name+'.bin')).write_bytes(binary);(out/(name+'.gltf')).write_text(json.dumps(r,indent=2)+'\n');del r['buffers'][0]['uri']
        j=json.dumps(r,separators=(',',':')).encode();j+=b' '*(-len(j)%4);b=bytes(binary)+b'\0'*(-len(binary)%4)
        (out/(name+'.glb')).write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
(out/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','authorship':'Original quad pair and directly encoded 32x32 black/white checker','oracle':'Camera z1 looks down at untextured floor; emitter z2 is only seen after scattering. With no propagated secondary derivatives, minification filter changes must not change pixels. Primary observation of the emitter must still honor minification.','artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir())]},indent=2)+'\n')
