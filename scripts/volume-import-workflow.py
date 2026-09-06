"""Native VOL import, analytic transport, failure atomicity, archive and recovery."""
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
p={'bounds':'file','reconstruction':'cell_constant_zero_outside','meters_per_unit':1,'density_scale':1,'emission_scale':[1,1,1],'absorption':[.5,1,2],'scattering':[0,0,0],'anisotropy':0,'max_step_meters':.1}
s=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];s.update(width=1,height=1,samples=1,max_depth=2,environment=[1,1,1]);s['light']['intensity']=[0,0,0];s['camera'].update(position=[1,1,5],target=[1,1,0],lens={'kind':'orthographic','xmag':1e-7,'ymag':1e-7,'near':.01,'far':20})
# Optical length is the integral of density along the test ray. Emission integrates separately.
cases=[('homogeneous','homogeneous',False,2,2,8),('emission','homogeneous',True,2,2,8),('axis','axis',False,6,2,8),('empty','empty',False,0,2,0),('emission-only','empty',True,0,2,8),('unit-cube','homogeneous',False,1,1,8),('scaled','homogeneous',False,4,4,8),('sparse','sparse',False,5/32,1/32,5)]
try:
 for name,file,emitting,optical,length,occupied in cases:
  policy=copy.deepcopy(p);settings=copy.deepcopy(s)
  if name in ['axis','unit-cube']:settings['camera'].update(position=[.5,.5,5],target=[.5,.5,0])
  if name=='unit-cube':policy['bounds']='unit_cube'
  if name=='scaled':policy['meters_per_unit']=2
  if name=='sparse':settings['camera'].update(position=[.984375,.984375,5],target=[.984375,.984375,0])
  source={'format':'vol','path':str(Path('fixtures/volume-import/data',file+'.vol').resolve()),'emission':str(Path('fixtures/volume-import/data/emission.vol').resolve()) if emitting else None,'policy':policy}
  project=root/(name+'-project');call(name+'-init',{'method':'init','document_id':f'{10100:032x}'},project);empty=call(name+'-empty',{'method':'inspect'},project)
  op={'method':'import','base_revision':empty['revision'],'idempotency_key':'volume:import:0001','max_added_bytes':8388608,'source':source,'settings':settings}
  imported=call(name+'-import',op,project,resource=True);assert imported['import']['occupied_cells']==occupied
  assert call(name+'-retry',op,project)==imported
  state=call(name+'-inspect',{'method':'inspect'},project);revision=state['revision']
  render={'method':'render','revision':revision,'backend':'cpu','output':str(root/(name+'-cpu'))};call(name+'-cpu',render,project,resource=True)
  rgb=pfm(root/(name+'-cpu/image/image.pfm'));expected=[]
  for sigma,e in zip(p['absorption'],[.25,.5,1]):
   t=math.exp(-sigma*optical);expected.append(t+(e*((1-t)/sigma if optical else length) if emitting else 0))
  assert max(abs(a-b) for a,b in zip(rgb,expected,strict=True))<2e-6,(name,rgb,expected)
  call(name+'-gpu',dict(render,backend='gpu',output=str(root/(name+'-gpu'))),project,resource=True)
  gpu=pfm(root/(name+'-gpu/image/image.pfm'));assert max(abs(a-b) for a,b in zip(gpu,expected,strict=True))<2e-5,(name,'GPU analytic',gpu,expected)
  archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
  restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored);call(name+'-recovered',dict(render,output=str(root/(name+'-recovered'))),restored)
  assert (root/(name+'-cpu/image/image.pfm')).read_bytes()==(root/(name+'-recovered/image/image.pfm')).read_bytes()
  call(name+'-recovered-gpu',dict(render,backend='gpu',output=str(root/(name+'-recovered-gpu'))),restored)
  assert (root/(name+'-gpu/image/image.pfm')).read_bytes()==(root/(name+'-recovered-gpu/image/image.pfm')).read_bytes()
  call(name+'-stale',dict(op,idempotency_key='volume:stale:0001'),project,error='stale_revision')
  if name=='homogeneous':
   unsupported=root/'scattering-project';call('scattering-init',{'method':'init','document_id':f'{10100:032x}'},unsupported);scattering_policy=dict(policy,scattering=[.1,0,0]);call('scattering-import',dict(op,source=dict(source,policy=scattering_policy)),unsupported);scattering_state=call('scattering-state',{'method':'inspect'},unsupported);call('scattering-gpu',dict(render,revision=scattering_state['revision'],backend='gpu',output=str(root/'scattering-gpu')),unsupported,error='unsupported_profile');assert call('scattering-unchanged',{'method':'inspect'},unsupported)['document_digest']==scattering_state['document_digest']
   for label,mutate,error in [('negative',lambda b:b.__setitem__(slice(48,52),struct.pack('<f',-1)),'vol'),('encoding',lambda b:b.__setitem__(4,2),'unsupported_vol'),('trailing',lambda b:b.append(0),'vol')]:
    b=bytearray(Path(source['path']).read_bytes());mutate(b);bad=root/(label+'.vol');bad.write_bytes(b);call(label,dict(op,base_revision=revision,idempotency_key='volume:'+label+':0001',source=dict(source,path=str(bad))),project,error=error)
   cancel=root/'cancel';cancel.write_text('cancel');call('cancel-import',dict(op,base_revision=revision),project,error='cancelled',limits={'cancel_file':str(cancel)})
   call('input-budget',dict(op,base_revision=revision),project,error='budget',limits={'max_input_bytes':1})
   fresh=root/'budget-project';call('budget-init',{'method':'init','document_id':f'{10100:032x}'},fresh);call('transaction-budget',dict(op,max_added_bytes=1),fresh,error='budget');assert call('budget-unchanged',{'method':'inspect'},fresh)['revision']==empty['revision']
  assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
  report['variants'].append({'name':name,'source':source,'settings':settings,'revision':revision,'report':imported['import'],'expected':expected,'archive':str(archive/'document/document.json'),'empty_revision':empty['revision']});save()
 report['status']='passed';save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
