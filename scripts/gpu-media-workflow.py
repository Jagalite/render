"""Native sparse media authoring, analytic CPU/Metal parity and archive recovery."""
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

identifier=lambda n:f'{n:032x}'
fixture=json.loads(Path('fixtures/gpu-media/data/cases.json').read_text())
def apply(name,project,state,commands):
 op={'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'gpu:media:'+name+':001','max_added_bytes':8388608,'commands':commands}}
 answer=call(name,op,project);assert call(name+'-retry',op,project)==answer
 call(name+'-stale',dict(op,request=dict(op['request'],idempotency_key='gpu:media:'+name+':stale')),project,error='stale_revision')
 return call(name+'-state',{'method':'inspect'},project)
try:
 for row in fixture['cases']:
  name=row['name'];project=root/(name+'-project')
  call(name+'-init',{'method':'init','document_id':identifier(10600)},project);state=call(name+'-empty',{'method':'inspect'},project);empty=state['revision']
  settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];settings.update(width=1,height=1,samples=1,max_depth=2,environment=fixture['environment']);settings['light']['intensity']=[0,0,0]
  camera=row['camera'];settings['camera'].update(position=camera['position'],target=camera['target'],lens={'kind':'orthographic','xmag':1e-7,'ymag':1e-7,'near':camera['near'],'far':camera['far']})
  commands=[{'operation':'set_render_settings','settings':settings}]
  for i,m in enumerate(row['media']):commands.extend([{'operation':'create_entity','entity':{'id':identifier(10601+i),'name':f'medium {i}','parent':None,'mesh':None,'material':None,'transform':m['transform']}},{'operation':'put_volume','asset':m['asset']}])
  state=apply(name+'-assets',project,state,commands)
  commands=[{'operation':'set_volume','entity':identifier(10601+i),'asset':next(k for k,v in state['snapshot']['volume_assets'].items() if v==m['asset'])} for i,m in enumerate(row['media'])]
  state=apply(name+'-attach',project,state,commands);revision=state['revision'];draw={'method':'render','revision':revision}
  errors={}
  for backend in ['cpu','gpu']:
   call(name+'-'+backend,dict(draw,backend=backend,output=str(root/(name+'-'+backend))),project,resource=True)
   rgb=pfm(root/(name+'-'+backend+'/image/image.pfm'));errors[backend]=max(abs(a-b) for a,b in zip(rgb,row['expected'],strict=True));assert errors[backend]<(2e-6 if backend=='cpu' else 2e-5),(name,backend,errors)
  passes=json.loads((root/(name+'-cpu/image/passes.json')).read_text());gp=json.loads((root/(name+'-gpu/image/passes.json')).read_text());assert passes[2:]==gp[2:]
  archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
  restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored)
  for backend in ['cpu','gpu']:
   call(name+'-recovered-'+backend,dict(draw,backend=backend,output=str(root/(name+'-recovered-'+backend))),restored);assert (root/(name+'-'+backend+'/image/image.pfm')).read_bytes()==(root/(name+'-recovered-'+backend+'/image/image.pfm')).read_bytes()
  call(name+'-stale-render',dict(draw,revision=empty,backend='gpu',output=str(root/(name+'-stale-render'))),project,error='stale_revision')
  if name=='homogeneous':
   cancel=root/'cancel';cancel.write_text('cancel');call('cancel-render',dict(draw,backend='gpu',output=str(root/'cancel-render')),project,error='cancelled',limits={'cancel_file':str(cancel)})
   call('byte-budget',dict(draw,backend='gpu',output=str(root/'byte-budget')),project,error='budget',limits={'max_output_bytes':1})
  assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
  report['variants'].append(dict(name=name,reference='analytic',revision=revision,settings=settings,archive=str(archive/'document/document.json'),expected=row['expected'],max_absolute_error=errors));save()
 for depth in [1,2,4,8]:
  name='surface-depth'+str(depth);project=root/(name+'-project');call(name+'-init',{'method':'init','document_id':identifier(10650)},project);state=call(name+'-empty',{'method':'inspect'},project)
  commands=json.loads(Path('fixtures/multibounce-pbr/create.json').read_text());settings=copy.deepcopy(commands[-1]['settings']);settings.update(samples=16,max_depth=depth,environment=[.1,.1,.1]);settings['light'].update(position=[.6,.3,1],intensity=[2,2,2]);commands[-1]['settings']=settings
  for command in commands:
   if command['operation']=='put_material' and command['material'].get('pbr') is None:command['material']['pbr']={'double_sided':False,'base_color':None,'metallic_roughness':None,'normal':None,'emission':None,'occlusion':None,'normal_scale':1,'occlusion_strength':1}
  asset={'origin':[-2,-2,-.2],'voxel_size':[4,4,2.4],'cells':[{'coordinate':[0,0,0],'density':1,'emission':[.01,.02,.03]}],'absorption':[.2,.4,.8],'scattering':[0,0,0],'anisotropy':0,'max_step_meters':.1}
  commands.extend([{'operation':'create_entity','entity':{'id':identifier(10651),'name':'room medium','parent':None,'mesh':None,'material':None,'transform':{'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}}},{'operation':'put_volume','asset':asset}]);state=apply(name+'-assets',project,state,commands)
  state=apply(name+'-attach',project,state,[{'operation':'set_volume','entity':identifier(10651),'asset':next(iter(state['snapshot']['volume_assets']))}]);revision=state['revision'];draw={'method':'render','revision':revision}
  for backend in ['cpu','gpu']:call(name+'-'+backend,dict(draw,backend=backend,output=str(root/(name+'-'+backend))),project,resource=True)
  cpu=pfm(root/(name+'-cpu/image/image.pfm'));gpu=pfm(root/(name+'-gpu/image/image.pfm'));error=max(abs(a-b) for a,b in zip(cpu,gpu,strict=True));assert error<2e-5,(name,error)
  passes=json.loads((root/(name+'-cpu/image/passes.json')).read_text());gp=json.loads((root/(name+'-gpu/image/passes.json')).read_text());assert passes[4]==gp[4] and any(x is not None for x in passes[4])
  archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project);restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored)
  for backend in ['cpu','gpu']:
   call(name+'-recovered-'+backend,dict(draw,backend=backend,output=str(root/(name+'-recovered-'+backend))),restored);assert (root/(name+'-'+backend+'/image/image.pfm')).read_bytes()==(root/(name+'-recovered-'+backend+'/image/image.pfm')).read_bytes()
  assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
  report['variants'].append(dict(name=name,reference='cpu',revision=revision,settings=settings,archive=str(archive/'document/document.json'),expected=cpu,max_absolute_error={'cpu':0,'gpu':error}));save()
 report['status']='passed';save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
