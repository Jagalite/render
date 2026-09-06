"""Original CC0 layered alpha fixture; writes a new directory, without a renderer."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import zlib

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=False)
binary = bytearray()
views = []
accessors = []
def view(data):
    binary.extend(b'\0' * (-len(binary) % 4))
    views.append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': len(data)})
    binary.extend(data)
    return len(views) - 1
def accessor(rows, kind, component, fmt):
    accessors.append({'bufferView': view(b''.join(struct.pack('<'+fmt, *row) for row in rows)),
                      'componentType': component, 'type': kind, 'count': len(rows)})
    return len(accessors) - 1
position = accessor([[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]], 'VEC3', 5126, '3f')
accessors[position].update(min=[-1,-1,0], max=[1,1,0])
normal = accessor([[0,0,1]]*4, 'VEC3', 5126, '3f')
uv = accessor([[0,1],[1,1],[1,0],[0,0]], 'VEC2', 5126, '2f')
indices = accessor([[0],[1],[2],[0],[2],[3]], 'SCALAR', 5123, 'H')
def chunk(kind, data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
png = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR',struct.pack('>IIBBBBB',4,1,8,6,0,0,0))
       + chunk(b'IDAT',zlib.compress(b'\0'+b''.join(bytes([255,255,255,a]) for a in [0,85,170,255])))
       + chunk(b'IEND',b''))
image_view = view(png)
primitive = {'attributes': {'POSITION':position,'NORMAL':normal,'TEXCOORD_0':uv}, 'indices':indices}
root = {'asset': {'version':'2.0','generator':'Original analytic alpha fixture'},
        'scene':0, 'scenes':[{'nodes':[0,1]}],
        'nodes':[{'name':'alpha-front','mesh':0,'translation':[0,0,1]}, {'name':'red-rear','mesh':1}],
        'meshes':[{'primitives':[dict(primitive,material=i)]} for i in range(2)],
        'materials':[{'name':'alpha-green','doubleSided':False,'alphaMode':'MASK',
                      'emissiveFactor':[0,1,0], 'pbrMetallicRoughness':{'baseColorFactor':[0,0,0,1],
                        'metallicFactor':0,'roughnessFactor':1,'baseColorTexture':{'index':0}}},
                     {'name':'opaque-red','emissiveFactor':[1,0,0],
                      'pbrMetallicRoughness':{'baseColorFactor':[0,0,0,1],'metallicFactor':0,'roughnessFactor':1}}],
        'images':[{'bufferView':image_view,'mimeType':'image/png'}],
        'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],
        'textures':[{'source':0,'sampler':0}], 'accessors':accessors, 'bufferViews':views,
        'buffers':[{'byteLength':len(binary)}]}
for name in ['mask','blend','opaque']:
    r = copy.deepcopy(root)
    r['materials'][0]['alphaMode'] = name.upper()
    if name == 'blend': r['materials'][0]['pbrMetallicRoughness']['baseColorFactor'][3] = 0.5
    r['buffers'][0]['uri'] = name+'.bin'
    (out/(name+'.gltf')).write_text(json.dumps(r,indent=2)+'\n')
    (out/(name+'.bin')).write_bytes(binary)
    del r['buffers'][0]['uri']
    j = json.dumps(r,separators=(',',':')).encode()
    j += b' '*(-len(j)%4)
    b = bytes(binary)+b'\0'*(-len(binary)%4)
    (out/(name+'.glb')).write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(b))
        +struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
provenance = {'license':'CC0-1.0', 'authorship':'Original analytic quads and directly encoded RGBA PNG',
              'oracle':'Orthographic stripes have alpha 0, 1/3, 2/3, 1; red rear, green front. At depth 1, R=1-coverage, G=coverage, B=0.',
              'artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir())]}
(out/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
