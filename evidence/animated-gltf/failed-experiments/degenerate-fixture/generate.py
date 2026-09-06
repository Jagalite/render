"""Original synthetic glTF fixture, authored directly from binary format fields.
Run explicitly to a NEW directory; never overwrites reviewed fixture bytes.
"""
import json, struct, sys
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
binary=bytearray();views=[];accessors=[]
def accessor(values,shape,component=5126,normalized=False):
    while len(binary)%4:binary.append(0)
    offset=len(binary)
    flat=[x for row in values for x in row]
    binary.extend(struct.pack('<'+{5126:'f',5123:'H',5121:'B'}[component]*len(flat),*flat))
    views.append({'buffer':0,'byteOffset':offset,'byteLength':len(binary)-offset})
    a={'bufferView':len(views)-1,'componentType':component,'count':len(values),'type':shape}
    if normalized:a['normalized']=True
    if shape=='SCALAR' and component==5126:a.update(min=[min(flat)],max=[max(flat)])
    if shape=='VEC3':a.update(min=[min(v[i] for v in values) for i in range(3)],max=[max(v[i] for v in values) for i in range(3)])
    accessors.append(a);return len(accessors)-1
position=accessor([[-.5,0,0],[.5,0,0],[-.5,2,0],[.5,2,0]],'VEC3')
indices=accessor([[0],[1],[2],[2],[1],[3]],'SCALAR',5123)
joints=accessor([[0,0,0,0],[0,0,0,0],[1,0,0,0],[1,0,0,0]],'VEC4',5121)
weights=accessor([[255,0,0,0]]*4,'VEC4',5121,True)
morph=accessor([[0,0,0],[0,0,0],[-.2,0,.2],[.2,0,.2]],'VEC3')
def inverse(x,y):return [1,0,0,0,0,1,0,0,0,0,1,0,-x,-y,0,1]
binds=accessor([inverse(.25,0),inverse(.25,1)],'MAT4')
times=accessor([[0],[1],[2]],'SCALAR')
rotation=accessor([[0,0,0,1],[0,0,2**-.5,2**-.5],[0,0,1,0]],'VEC4')
morph_values=accessor([[.25],[1],[0]],'SCALAR')
translation=accessor([[0,0,0],[.25,0,0],[.2,0,0],[.2,0,0],[.45,0,0],[.2,0,0],[.2,0,0],[.65,0,0],[0,0,0]],'VEC3')
root={'asset':{'version':'2.0','generator':'Original independent fixture generator'},'buffers':[{'uri':'character.bin','byteLength':len(binary)}],'bufferViews':views,'accessors':accessors,
'materials':[{'pbrMetallicRoughness':{'baseColorFactor':[.08,.35,.8,1],'metallicFactor':0,'roughnessFactor':.8},'doubleSided':True}],
'meshes':[{'weights':[.25],'primitives':[{'attributes':{'POSITION':position,'JOINTS_0':joints,'WEIGHTS_0':weights},'indices':indices,'material':0,'targets':[{'POSITION':morph}]}]}],
'nodes':[{'name':'hip','translation':[.25,0,0],'children':[1]},{'name':'shoulder','translation':[0,1,0]},{'name':'character','mesh':0,'skin':0,'translation':[5,0,0]}],
'skins':[{'joints':[0,1],'skeleton':0,'inverseBindMatrices':binds}],
'animations':[{'name':'bend','samplers':[{'input':times,'output':rotation,'interpolation':'LINEAR'},{'input':times,'output':morph_values,'interpolation':'STEP'},{'input':times,'output':translation,'interpolation':'CUBICSPLINE'}],'channels':[{'sampler':0,'target':{'node':1,'path':'rotation'}},{'sampler':1,'target':{'node':2,'path':'weights'}},{'sampler':2,'target':{'node':0,'path':'translation'}}]}],
'scenes':[{'nodes':[0,2]}],'scene':0}
(out/'character.gltf').write_text(json.dumps(root,indent=2)+'\n');(out/'character.bin').write_bytes(binary)
root['buffers'][0].pop('uri');jb=json.dumps(root,separators=(',',':')).encode();jb+=b' '*((-len(jb))%4);bb=bytes(binary);bb+=b'\0'*((-len(bb))%4)
glb=struct.pack('<III',0x46546c67,2,12+8+len(jb)+8+len(bb))+struct.pack('<II',len(jb),0x4e4f534a)+jb+struct.pack('<II',len(bb),0x004e4942)+bb
(out/'character.glb').write_bytes(glb)
