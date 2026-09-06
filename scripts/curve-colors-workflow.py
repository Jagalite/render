"""Native polyline RGBA, groom transfer, CPU/Metal parity and evaluated GLB roundtrip."""
import copy,json,math,re,struct,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
report={'status':'in_progress','calls':[],'variants':[]}
def save():(root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name,op,target,error=None,limits=None,resource=False):
 q={'version':0,'operation':op}
 if limits is not None:q['limits']=limits
 p=root/(f'{len(report["calls"]):03}-'+name+'.request.json');p.write_text(json.dumps(q,indent=2)+'\n')
 cmd=['target/debug/render-host','project',str(target),str(p)]
 if resource and sys.platform=='darwin':cmd=['/usr/bin/time','-l']+cmd
 at=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,timeout=90)
 p.with_suffix('.stdout.json').write_text(r.stdout);p.with_suffix('.stderr.txt').write_text(r.stderr)
 row={'name':name,'command':cmd,'exit_code':r.returncode,'expected_error':error,'seconds':time.monotonic()-at}
 if resource and not r.returncode and sys.platform=='darwin':row['resources']={label:int(re.search(r'(\d+)\s+'+label,r.stderr).group(1)) for label in ['maximum resident set size','peak memory footprint']}
 report['calls'].append(row);save()
 if error:assert r.returncode!=0 and not r.stdout,(name,r);v=json.loads(r.stderr);assert v['code']==error,(name,v)
 else:assert r.returncode==0,(name,r.stderr);v=json.loads(r.stdout)
 print(name+': passed',flush=True);return v
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
p={'byte_order':'little_endian','meters_per_unit':1,'thickness':'diameter','color_space':'linear_srgb','transparency':'coverage_one_minus_transparency','representation':'swept_polyline_surface','roughness':.6,'tessellation':{'chord_error':.001,'radial_error':.01,'max_samples':8192,'max_vertices':65536,'max_depth':18},'point_attributes':'linear_rgba_f32'}
s=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];s.update(width=16,height=16,samples=16,max_depth=2,environment=[.1,.1,.1]);s['light'].update(position=[1,2,3],intensity=[4,4,4]);s['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':1,'ymag':1,'near':.01,'far':20})
export_policy={'allow_approximations':True,'max_bytes':4194304,'max_vertices':10000,'max_position_error_meters':1e-5}
def finish(name,project,state,settings,row):
 revision=state['revision'];draw={'method':'render','revision':revision}
 for backend in ['cpu','gpu']:call(name+'-'+backend,dict(draw,backend=backend,output=str(root/(name+'-'+backend))),project,resource=True)
 cpu=pfm(root/(name+'-cpu/image/image.pfm'));gpu=pfm(root/(name+'-gpu/image/image.pfm'));rmse=(sum((a-b)**2 for a,b in zip(cpu,gpu,strict=True))/len(cpu))**.5;assert rmse<.002,(name,rmse)
 passes=json.loads((root/(name+'-cpu/image/passes.json')).read_text());gp=json.loads((root/(name+'-gpu/image/passes.json')).read_text());assert passes[4]==gp[4] and any(x is not None for x in passes[4]);assert any(abs(x-settings['environment'][i%3])>.005 for i,x in enumerate(cpu))
 archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
 restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored)
 for backend in ['cpu','gpu']:
  call(name+'-recovered-'+backend,dict(draw,backend=backend,output=str(root/(name+'-recovered-'+backend))),restored);assert (root/(name+'-'+backend+'/image/image.pfm')).read_bytes()==(root/(name+'-recovered-'+backend+'/image/image.pfm')).read_bytes()
 output=root/(name+'-export');export_op={'method':'export','revision':revision,'output':str(output),'content':{'format':'glb','policy':export_policy}};call(name+'-export',export_op,project)
 glb=output/'scene/scene.glb';b=glb.read_bytes();length=int.from_bytes(b[12:16],'little');gltf=json.loads(b[20:20+length]);assert any('COLOR_0' in primitive['attributes'] for mesh in gltf['meshes'] for primitive in mesh['primitives'])
 call(name+'-repeat',dict(export_op,output=str(root/(name+'-repeat'))),project);assert b==(root/(name+'-repeat/scene/scene.glb')).read_bytes()
 round_project=root/(name+'-round-project');call(name+'-round-init',{'method':'init','document_id':f'{10400:032x}'},round_project);empty=call(name+'-round-empty',{'method':'inspect'},round_project)
 imported=call(name+'-round-import',{'method':'import','base_revision':empty['revision'],'idempotency_key':'curve:rgba:round:001','max_added_bytes':8388608,'source':{'format':'glb','path':str(glb),'policy':{'allow_approximations':True}},'settings':settings},round_project)
 round_state=call(name+'-round-state',{'method':'inspect'},round_project);exported=json.loads((output/'scene/report.json').read_text());mapping={}
 for old,node in exported['source_entities'].items():
  parent=imported['import']['source_nodes'][str(node)];leaves=[e['id'] for e in round_state['snapshot']['entities'] if e['parent']==parent and e['mesh'] is not None];assert len(leaves)==1;mapping[old]=leaves[0]
 roundtrip=[]
 for backend in ['cpu','gpu']:
  call(name+'-round-'+backend,{'method':'render','revision':round_state['revision'],'backend':backend,'output':str(root/(name+'-round-'+backend))},round_project)
  rgb=pfm(root/(name+'-round-'+backend+'/image/image.pfm'));error=(sum((a-b)**2 for a,b in zip(cpu,rgb,strict=True))/len(cpu))**.5;assert error<.002,(name,backend,error)
  rp=json.loads((root/(name+'-round-'+backend+'/image/passes.json')).read_text());assert [mapping.get(x) if x is not None else None for x in passes[4]]==rp[4];roundtrip.append({'backend':backend,'linear_rmse':error})
 assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
 row.update(name=name,settings=settings,revision=revision,archive=str(archive/'document/document.json'),glb=str(glb),export_policy=export_policy,gpu_linear_rmse=rmse,roundtrip=roundtrip);report['variants'].append(row);save()
try:
 for name,file in [('gradient-le','gradient-le'),('gradient-be','gradient-be'),('rgb-only','rgb-only'),('alpha-only','alpha-only'),('mixed-opacity','mixed-opacity'),('srgb','gradient-le')]:
  policy=copy.deepcopy(p);settings=copy.deepcopy(s)
  if file.endswith('-be'):policy['byte_order']='big_endian'
  if name=='srgb':policy['color_space']='srgb'
  source={'format':'hair','path':str(Path('fixtures/curve-colors/data',file+'.hair').resolve()),'policy':policy};project=root/(name+'-project')
  call(name+'-init',{'method':'init','document_id':f'{10400:032x}'},project);empty=call(name+'-empty',{'method':'inspect'},project)
  op={'method':'import','base_revision':empty['revision'],'idempotency_key':'curve:rgba:import:001','max_added_bytes':8388608,'source':source,'settings':settings};imported=call(name+'-import',op,project,resource=True);assert call(name+'-retry',op,project)==imported
  state=call(name+'-state',{'method':'inspect'},project);assert state['snapshot']['version']==15 and imported['import']['profile']=='hair-polyline-rgba-v1';assert imported['import']['material_groups']==(2 if name=='mixed-opacity' else 1)
  observed=call(name+'-evaluate',{'method':'evaluate','revision':state['revision']},project);assert sum(c['derived_bytes'] for c in observed['conversions'])==imported['import']['observed_derived_json_bytes']
  call(name+'-stale',dict(op,idempotency_key='curve:rgba:stale:001'),project,error='stale_revision')
  if name=='gradient-le':
   strict=copy.deepcopy(policy);strict.pop('point_attributes');call('strict-profile',dict(op,base_revision=state['revision'],source=dict(source,policy=strict)),project,error='unsupported_hair')
   b=bytearray(Path(source['path']).read_bytes());b[216:220]=struct.pack('<f',0);bad=root/'zero-radius.hair';bad.write_bytes(b);call('zero-radius',dict(op,base_revision=state['revision'],source=dict(source,path=str(bad))),project,error='curve')
   bad_policy=copy.deepcopy(policy);bad_policy['tessellation']['max_vertices']=6;call('sweep-budget',dict(op,base_revision=state['revision'],source=dict(source,policy=bad_policy)),project,error='budget')
   cancel=root/'cancel';cancel.write_text('cancel');call('cancel-import',dict(op,base_revision=state['revision']),project,error='cancelled',limits={'cancel_file':str(cancel)})
   call('input-budget',dict(op,base_revision=state['revision']),project,error='budget',limits={'max_input_bytes':1})
  finish(name,project,state,settings,{'kind':'hair','source':source,'report':imported['import'],'empty_revision':empty['revision']})
 # Native guide and child paths share the same point attribute consumer.
 name='groom';source=Path('evidence/m06-m08/groom-01/document.json').resolve();project=root/(name+'-project');call('groom-restore-source',{'method':'restore','source':str(source)},project);state=call('groom-source-state',{'method':'inspect'},project);groom_id=f'{7601:032x}';g=copy.deepcopy(state['snapshot']['grooms'][groom_id]);g['children']=g['children'][:2]
 for guide in g['guides']:
  guide['curve']['basis']={'kind':'polyline'}
  for i,c in enumerate(guide['curve']['controls']):c['color']=[1-i/3,.2,i/3,1-.6*i/3];c['radius']*=2
 settings=copy.deepcopy(state['snapshot']['render_settings']);settings.update(width=32,height=32,samples=16,max_depth=2)
 commands=[{'operation':'set_groom','entity':groom_id,'groom':g},{'operation':'set_render_settings','settings':settings}]
 for material in state['snapshot']['materials'].values():
  m=copy.deepcopy(material);m['pbr']={'double_sided':True,'base_color':None,'metallic_roughness':None,'normal':None,'emission':None,'occlusion':None,'normal_scale':1,'occlusion_strength':1,'advanced':{'model':{'kind':'principled'},'opacity':{'kind':'opaque'}}}
  if m['id']==next(e['material'] for e in state['snapshot']['entities'] if e['id']==groom_id):m.update(base_color=[1,1,1],roughness=.6);m['pbr']['advanced']['opacity']={'kind':'blend','factor':1}
  commands.append({'operation':'put_material','material':m})
 op={'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'curve:rgba:groom:001','max_added_bytes':8388608,'commands':commands}};applied=call('groom-author',op,project);assert call('groom-retry',op,project)==applied
 state=call('groom-authored',{'method':'inspect'},project);assert state['snapshot']['version']==15
 finish(name,project,state,settings,{'kind':'groom','source_document':str(source)})
 for backend in ['cpu','gpu']:assert (root/('gradient-le-'+backend+'/image/image.pfm')).read_bytes()==(root/('gradient-be-'+backend+'/image/image.pfm')).read_bytes()
 report['status']='passed';save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
