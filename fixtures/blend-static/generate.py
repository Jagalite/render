"""Original CC0 analytic static containers, independent of Blender implementation.
These exercise the declared SDNA profile; real-file qualification uses a separately
pinned official startup.blend. This generator is development tooling only.
"""
from pathlib import Path
import struct,json,hashlib,sys,re
out=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).parent/'data';out.mkdir(parents=True,exist_ok=True)
S={}
def spec(name,fields):S[name]=[(t,n) for t,n in fields]
def pointers(names,t='void'):return [(t,'*'+n) for n in names.split()]
spec('ID',pointers('lib override_library')+[('char','name[66]')])
spec('ListBase',pointers('first last'))
spec('PartDeflect',[('short','deflect'),('short','forcefield')])
spec('UnitSettings',[('float','scale_length')])
spec('RenderData',[('float','xasp'),('float','yasp'),('int','xsch'),('int','ysch')])
spec('Scene',[('ID','id')]+pointers('adt set gpd rigidbody_world ed camera world master_collection')+[('char','use_nodes'),('UnitSettings','unit'),('RenderData','r'),('ListBase','view_layers')])
spec('Collection',[('ID','id'),('short','flag'),('ListBase','children'),('ListBase','gobject')])
spec('CollectionChild',pointers('next prev collection'))
spec('CollectionObject',pointers('next prev ob'))
spec('ViewLayer',[('short','flag')]+pointers('next prev mat_override')+[('ListBase','layer_collections')])
spec('LayerCollection',pointers('next prev collection')+[('short','flag'),('ListBase','layer_collections')])
obj_none='adt track proxy proxy_group proxy_from ipo action poselib pose gpd pd soft dup_group fluidsimSettings rigidbody_object rigidbody_constraint'
obj_lists='modifiers greasepencil_modifiers shader_fx constraints constraintChannels effect nlastrips hooks particlesystem'
spec('Object',[('ID','id')]+pointers(obj_none+' parent data matbits')+[('Material','**mat')]+[('ListBase',n) for n in obj_lists.split()]+[('short',n) for n in ['partype','transflag','restrictflag','rotmode','type']]+[('int','totcol'),('int','mode')]+[('float',n+'[3]') for n in ['dloc','drot','dscale','loc','rot','size']]+[('float','dquat[4]'),('float','quat[4]'),('float','drotAngle'),('float','parentinv[4][4]')])
spec('CustomData',[('int','totlayer')]+pointers('layers'))
spec('CustomDataLayer',[('int','type'),('char','name[64]')]+pointers('data'))
spec('Mesh',[('ID','id')]+pointers('adt ipo key texcomesh dvert mvert medge mloop mpoly')+[('Material','**mat')]+[('int',n) for n in ['totface','totvert','totedge','totpoly','totloop','totcol']]+[('CustomData','ldata')])
spec('MVert',[('float','co[3]')]);spec('MEdge',[('uint','v1'),('uint','v2')]);spec('MLoop',[('uint','v'),('uint','e')]);spec('MPoly',[('int','loopstart'),('int','totloop'),('short','mat_nr'),('char','flag')]);spec('MLoopUV',[('float','uv[2]')])
spec('Material',[('ID','id')]+pointers('adt ipo gp_style nodetree')+[('char',n) for n in ['use_nodes','blend_method','blend_shadow','blend_flag']])
spec('bNodeTree',[('ID','id')]+pointers('adt')+[('ListBase','nodes'),('ListBase','links')])
spec('bNode',pointers('next prev id storage')+[('char','idname[64]'),('int','flag'),('short','custom1'),('ListBase','internal_links'),('ListBase','inputs'),('ListBase','outputs')])
spec('bNodeSocket',pointers('next prev link default_value')+[('char','identifier[64]'),('short','type')])
spec('bNodeLink',pointers('next prev fromnode tonode fromsock tosock')+[('int','flag')])
spec('bNodeSocketValueFloat',[('float','value')]);spec('bNodeSocketValueRGBA',[('float','value[4]')]);spec('bNodeSocketValueVector',[('float','value[3]')])
spec('CameraDOFSettings',[('short','flag')])
spec('Camera',[('ID','id')]+pointers('adt ipo')+[('float',n) for n in ['shiftx','shifty','clipsta','clipend','sensor_x','sensor_y','lens','ortho_scale']]+[('char','sensor_fit'),('char','type'),('CameraDOFSettings','dof')])
spec('Lamp',[('ID','id')]+pointers('adt ipo')+[('short','type'),('short','use_nodes')]+[('float',n) for n in ['energy','soft','r','g','b']])
spec('World',[('ID','id')]+pointers('adt ipo nodetree')+[('short','use_nodes')])
manifest=[]
for width in [4,8]:
 for order in ['little','big']:
  prefix='<' if order=='little' else '>';p=lambda fmt,*v:struct.pack(prefix+fmt,*v)
  basic={'char':('b',1),'short':('h',2),'int':('i',4),'uint':('I',4),'float':('f',4),'void':('',0)}
  names=list(dict.fromkeys(n for fields in S.values() for t,n in fields));types=list(basic)+list(S);sizes={t:v[1] for t,v in basic.items()}
  def shape(n):return n.lstrip('*').split('[')[0],__import__('math').prod(map(int,re.findall(r'\[(\d+)\]',n)))
  for t,fields in S.items():sizes[t]=sum((width if n.startswith('*') else sizes[ft])*shape(n)[1] for ft,n in fields)
  def encode(t,values):
   data=bytearray()
   for ft,n in S[t]:
    key,count=shape(n);value=values.get(key)
    if n.startswith('*'):data+=p(('I' if width==4 else 'Q')*count,*(([0]*count) if value is None else ([value] if count==1 else value)))
    elif ft in S:data+=encode(ft,value or {})
    elif ft=='char' and '[' in n:data+=(value or '').encode().ljust(count,b'\0')[:count]
    else:data+=p(basic[ft][0]*count,*(([0]*count) if value is None else ([value] if count==1 else value)))
   assert len(data)==sizes[t];return data
  blocks=[];addresses={};next_address=0x1000
  def alloc(label):
   global next_address
   if label not in addresses:addresses[label]=next_address;next_address+=0x100
   return addresses[label]
  def add(label,t,values,code=b'DATA'):
   if isinstance(values,dict):values=[values]
   blocks.append((code,alloc(label),list(S).index(t),len(values),b''.join(encode(t,v) for v in values)))
  def raw(label,data):blocks.append((b'DATA',alloc(label),0,1,data))
  def head(labels):return {'first':alloc(labels[0]),'last':alloc(labels[-1])} if labels else {}
  def chain(labels,t,values):
   for i,(label,value) in enumerate(zip(labels,values)):add(label,t,{'next':alloc(labels[i+1]) if i+1<len(labels) else 0,'prev':alloc(labels[i-1]) if i else 0,**value})
  add('scene','Scene',{'id':{'name':'SCScene'},'camera':alloc('camera-object'),'world':alloc('world'),'master_collection':alloc('collection'),'unit':{'scale_length':1},'r':{'xasp':1,'yasp':1,'xsch':64,'ysch':64},'view_layers':head(['view'])},b'SC\0\0')
  add('collection','Collection',{'id':{'name':'COMain'},'gobject':head(['member-parent','member-mesh','member-camera','member-light'])})
  chain(['member-parent','member-mesh','member-camera','member-light'],'CollectionObject',[{'ob':alloc(n)} for n in ['parent-object','mesh-object','camera-object','light-object']])
  add('view','ViewLayer',{'flag':1,'layer_collections':head(['layer'])});add('layer','LayerCollection',{'collection':alloc('collection')})
  base={'dscale':[1]*3,'dquat':[1,0,0,0],'quat':[1,0,0,0],'size':[1]*3,'rotmode':1,'parentinv':[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}
  add('parent-object','Object',{**base,'id':{'name':'OBParent'},'loc':[1,0,0]})
  add('mesh-object','Object',{**base,'id':{'name':'OBQuad'},'pd':alloc('inactive-field'),'parent':alloc('parent-object'),'data':alloc('mesh'),'type':1,'totcol':1,'matbits':alloc('matbits'),'mat':alloc('materials'),'loc':[-1,0,0]})
  add('camera-object','Object',{**base,'id':{'name':'OBCamera'},'data':alloc('camera'),'type':11,'loc':[0,0,4]})
  add('light-object','Object',{**base,'id':{'name':'OBLight'},'data':alloc('lamp'),'type':10,'loc':[0,0,3]})
  add('inactive-field','PartDeflect',{});raw('matbits',bytes([0,0,0,0]));raw('materials',p('I' if width==4 else 'Q',alloc('material')))
  add('mesh','Mesh',{'id':{'name':'MEQuad'},'mvert':alloc('vertices'),'medge':alloc('edges'),'mloop':alloc('loops'),'mpoly':alloc('polygons'),'mat':alloc('materials'),'totvert':4,'totedge':4,'totpoly':1,'totloop':4,'totcol':1,'ldata':{'totlayer':1,'layers':alloc('uv-layer')}})
  add('vertices','MVert',[{'co':v} for v in [[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]]]);add('edges','MEdge',[{'v1':i,'v2':(i+1)%4} for i in range(4)]);add('loops','MLoop',[{'v':i,'e':i} for i in range(4)]);add('polygons','MPoly',{'totloop':4})
  add('uv-layer','CustomDataLayer',{'type':16,'name':'UVMap','data':alloc('uv')});add('uv','MLoopUV',[{'uv':v} for v in [[0,0],[1,0],[1,1],[0,1]]])
  add('camera','Camera',{'id':{'name':'CACamera'},'clipsta':0.1,'clipend':100,'sensor_x':36,'sensor_y':24,'lens':50,'ortho_scale':4})
  add('lamp','Lamp',{'id':{'name':'LALight'},'energy':32,'r':1,'g':0.5,'b':0.25})
  add('material','Material',{'id':{'name':'MAMaterial'},'use_nodes':1,'blend_shadow':1,'nodetree':alloc('material-tree')})
  add('world','World',{'id':{'name':'WOWorld'},'use_nodes':1,'nodetree':alloc('world-tree')})
  def node_graph(prefix,shader_kind,output_kind,inputs,output_name):
   shader=prefix+'-shader';output=prefix+'-output';link=prefix+'-link';src=prefix+'-src';dst=prefix+'-dst'
   add(prefix+'-tree','bNodeTree',{'id':{'name':'NT'+prefix},'nodes':head([shader,output]),'links':head([link])})
   labels=[prefix+'-input-'+str(i) for i in range(len(inputs))]
   chain([shader,output],'bNode',[{'idname':shader_kind,'inputs':head(labels),'outputs':head([src])},{'idname':output_kind,'inputs':head([dst])}])
   chain(labels,'bNodeSocket',[{'identifier':key,'type':kind,'default_value':alloc(label+'-value')} for label,(key,kind,value) in zip(labels,inputs)])
   for label,(key,kind,value) in zip(labels,inputs):add(label+'-value',{0:'bNodeSocketValueFloat',1:'bNodeSocketValueVector',2:'bNodeSocketValueRGBA'}[kind],{'value':value})
   add(src,'bNodeSocket',{'identifier':output_name,'type':3});add(dst,'bNodeSocket',{'identifier':'Surface','type':3,'link':alloc(link)})
   add(link,'bNodeLink',{'fromnode':alloc(shader),'tonode':alloc(output),'fromsock':alloc(src),'tosock':alloc(dst),'flag':2})
  floats={'Subsurface':0,'Metallic':0,'Specular':0.5,'Specular Tint':0,'Roughness':0.5,'Anisotropic':0,'Anisotropic Rotation':0,'Sheen':0,'Sheen Tint':0.5,'Clearcoat':0,'Clearcoat Roughness':0.03,'IOR':1.45,'Transmission':0,'Transmission Roughness':0,'Emission Strength':1,'Alpha':1}
  inputs=[('Base Color',2,[0.5,0.25,0.125,1]),('Subsurface Radius',1,[1,0.2,0.1]),('Subsurface Color',2,[0.5,0.25,0.125,1]),('Emission',2,[0,0,0,1])]+[(k,0,v) for k,v in floats.items()]+[(k,1,[0,0,0]) for k in ['Normal','Clearcoat Normal','Tangent']]
  node_graph('material','ShaderNodeBsdfPrincipled','ShaderNodeOutputMaterial',inputs,'BSDF');node_graph('world','ShaderNodeBackground','ShaderNodeOutputWorld',[('Color',2,[0.125,0.25,0.5,1]),('Strength',0,0.5)],'Background')
  dna=bytearray(b'SDNANAME'+p('I',len(names))+b'\0'.join(n.encode() for n in names)+b'\0');dna+=bytes((-len(dna))%4);dna+=b'TYPE'+p('I',len(types))+b'\0'.join(t.encode() for t in types)+b'\0';dna+=bytes((-len(dna))%4);dna+=b'TLEN'+p('H'*len(types),*(sizes[t] for t in types));dna+=bytes((-len(dna))%4);dna+=b'STRC'+p('I',len(S))
  for t,fields in S.items():dna+=p('HH',types.index(t),len(fields))+b''.join(p('HH',types.index(ft),names.index(n)) for ft,n in fields)
  blocks+=[(b'DNA1',0,0,1,dna),(b'ENDB',0,0,0,b'')]
  data=b'BLENDER'+(b'_' if width==4 else b'-')+(b'v' if order=='little' else b'V')+b'293'
  offsets={}
  for code,token,sdna,count,body in blocks:
   offsets[str(token)]=len(data)+16+width;data+=p('4sI'+('I' if width==4 else 'Q')+'II',code,len(body),token,sdna,count)+body
  filename=f'quad-{width*8}-{order}.blend';(out/filename).write_bytes(data)
  manifest.append({'file':filename,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'pointer_bytes':width,'byte_order':order})
  layout={}
  for t,fields in S.items():
   at=0;layout[t]={}
   for ft,n in fields:
    key,count=shape(n);layout[t][key]=at;at+=(width if n.startswith('*') else sizes[ft])*count
  (out/(filename+'.layout.json')).write_text(json.dumps({'addresses':addresses,'data_offsets':offsets,'fields':layout,'sizes':sizes},indent=2)+'\n')
# A nontrivial three-axis authored camera rotation probes persisted math parity.
layout=json.loads((out/'quad-64-little.blend.layout.json').read_text());data=bytearray((out/'quad-64-little.blend').read_bytes());token=layout['addresses']['camera-object'];at=layout['data_offsets'][str(token)]+layout['fields']['Object']['rot'];data[at:at+12]=struct.pack('<3f',0.1,0.2,0.3)
name='quad-tilted.blend';(out/name).write_bytes(data);(out/(name+'.layout.json')).write_text(json.dumps(layout,indent=2)+'\n');manifest.append({'file':name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'pointer_bytes':8,'byte_order':'little','scenario':'Nontrivial authored XYZ camera rotation'})
# A second object references the same source mesh/material, without coplanar overlap.
layout=json.loads((out/'quad-64-little.blend.layout.json').read_text());data=bytearray((out/'quad-64-little.blend').read_bytes());base=layout['data_offsets'][str(layout['addresses']['parent-object'])]
for field,value,fmt in [('type',1,'h'),('totcol',1,'i'),('data',layout['addresses']['mesh'],'Q'),('matbits',layout['addresses']['matbits'],'Q'),('mat',layout['addresses']['materials'],'Q')]:
 at=base+layout['fields']['Object'][field];encoded=struct.pack('<'+fmt,value);data[at:at+len(encoded)]=encoded
for label,x in [('parent-object',2.25),('mesh-object',-2.25)]:
 at=layout['data_offsets'][str(layout['addresses'][label])]+layout['fields']['Object']['loc'];data[at:at+4]=struct.pack('<f',x)
name='quad-instances.blend';(out/name).write_bytes(data);(out/(name+'.layout.json')).write_text(json.dumps(layout,indent=2)+'\n');manifest.append({'file':name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'pointer_bytes':8,'byte_order':'little','scenario':'Two spatial instances share one authored mesh and material'})
(out/'provenance.json').write_text(json.dumps({'author':'Render project','license':'CC0-1.0','purpose':'Independent analytic SDNA static profile corpus, not evidence that Blender itself accepts these minimal synthetic containers','files':manifest},indent=2)+'\n')
