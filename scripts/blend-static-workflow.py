"""Native transaction/import/render/source-recovery gate; optional pinned real input."""
import copy,hashlib,json,math,os,re,struct,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=False)
report={'status':'in_progress','calls':[],'variants':[]}
def save(): (root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name,op,project,error=None,limits=None,resource=False,binary='target/debug/render-host'):
 q={'version':0,'operation':op}
 if limits is not None:q['limits']=limits
 path=root/(f'{len(report["calls"]):03}-{name}.request.json');path.write_text(json.dumps(q,indent=2)+'\n')
 cmd=[binary,'project',str(project),str(path)]
 if resource and sys.platform=='darwin':cmd=['/usr/bin/time','-l']+cmd
 at=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,timeout=120)
 path.with_suffix('.stdout.json').write_text(r.stdout);path.with_suffix('.stderr.txt').write_text(r.stderr)
 row={'name':name,'command':cmd,'exit_code':r.returncode,'expected_error':error,'seconds':time.monotonic()-at}
 if resource and not r.returncode and sys.platform=='darwin':row['resources']={label:int(re.search(r'(\d+)\s+'+label,r.stderr).group(1)) for label in ['maximum resident set size','peak memory footprint']}
 report['calls'].append(row);save()
 if error:
  assert r.returncode!=0 and not r.stdout,(name,r);v=json.loads(r.stderr);assert v['code']==error,(name,v)
 else:assert r.returncode==0,(name,r.stderr);v=json.loads(r.stdout)
 print(name+': passed',flush=True);return v

def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
policy={'scene':'Scene','meters_per_unit':1,'allow_principled_approximation':True,'allow_point_light_approximation':True}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];settings.update(width=32,height=32,samples=4,max_depth=4)
cases=[(f'quad-{w}-{o}',Path(f'fixtures/blend-static/data/quad-{w}-{o}.blend'),False) for w in [32,64] for o in ['little','big']]
cases.append(('quad-tilted',Path('fixtures/blend-static/data/quad-tilted.blend'),False))
cases.append(('quad-instances',Path('fixtures/blend-static/data/quad-instances.blend'),False))
if os.environ.get('RENDER_BLEND_REFERENCE_PATH'):cases.append(('startup-293',Path(os.environ['RENDER_BLEND_REFERENCE_PATH']),True))
try:
 for name,path,external in cases:
  original=path.read_bytes();source={'format':'blend','path':str(path.resolve()),'policy':copy.deepcopy(policy)};s=copy.deepcopy(settings)
  if external:
   assert hashlib.sha256(original).hexdigest()=='339bc1c5cdc7fb0f3a9250f66d9616b4511e42bf08feb31ccf01344b5db655ea';s.update(height=18)
  project=root/(name+'-project');call(name+'-init',{'method':'init','document_id':f'{293:032x}'},project);empty=call(name+'-empty',{'method':'inspect'},project)
  operation={'method':'import','base_revision':empty['revision'],'idempotency_key':'blend:import:0001','max_added_bytes':8388608,'source':source,'settings':s}
  imported=call(name+'-import',operation,project,resource=True);assert call(name+'-retry',operation,project)==imported
  state=call(name+'-state',{'method':'inspect'},project);revision=state['revision'];asset=imported['import']['source_asset'];snap=state['snapshot'];assert snap['version']==16 and bytes(snap['source_assets'][asset]['bytes'])==original
  mesh=next(iter(snap['meshes'].values()));assert len(mesh['positions']['values'])==(8 if external else 4);assert len(mesh['corners'])==(24 if external else 4)
  assert len(snap['entities'])==(3 if external else 4)
  assert len(snap['meshes'])==1 and len(snap['materials'])==1
  if name=='quad-instances':assert sum(e['mesh'] is not None for e in snap['entities'])==2
  render={'method':'render','revision':revision,'backend':'cpu','output':str(root/(name+'-cpu'))};call(name+'-cpu',render,project,resource=True)
  call(name+'-gpu',dict(render,backend='gpu',output=str(root/(name+'-gpu'))),project,resource=True)
  cpu=pfm(root/(name+'-cpu/image/image.pfm'));gpu=pfm(root/(name+'-gpu/image/image.pfm'));rmse=math.sqrt(sum((a-b)**2 for a,b in zip(cpu,gpu,strict=True))/len(cpu));assert rmse<5e-5,(name,rmse)
  archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
  exported=root/(name+'-source');export={'method':'export','revision':revision,'output':str(exported),'content':{'format':'source','asset':asset}};call(name+'-source',export,project);assert (exported/'source/original.blend').read_bytes()==original
  restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored);restored_state=call(name+'-restored-state',{'method':'inspect'},restored);assert restored_state['revision']==revision
  call(name+'-recovered',dict(render,output=str(root/(name+'-recovered'))),restored);assert (root/(name+'-cpu/image/image.pfm')).read_bytes()==(root/(name+'-recovered/image/image.pfm')).read_bytes()
  call(name+'-recovered-gpu',dict(render,backend='gpu',output=str(root/(name+'-recovered-gpu'))),restored);assert (root/(name+'-gpu/image/image.pfm')).read_bytes()==(root/(name+'-recovered-gpu/image/image.pfm')).read_bytes()
  call(name+'-recovered-source',dict(export,output=str(root/(name+'-recovered-source'))),restored);assert (root/(name+'-recovered-source/source/original.blend')).read_bytes()==original
  call(name+'-stale',dict(operation,idempotency_key='blend:stale:00001'),project,error='stale_revision')
  call(name+'-stale-export',dict(export,revision=empty['revision'],output=str(root/(name+'-stale-export'))),project,error='stale_revision')
  assert not (root/(name+'-stale-export')).exists()
  assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
  if name=='quad-64-little':
   for label,mutate,code in [('version',lambda b:b.__setitem__(slice(9,12),b'292'),'unsupported_blend'),('trailing',lambda b:b.append(0),'blend'),('compressed',lambda b:b.__setitem__(slice(0,2),bytes([31,139])),'unsupported_blend')]:
    bad=bytearray(original);mutate(bad);badpath=root/(label+'.blend');badpath.write_bytes(bad);call(label,dict(operation,source=dict(source,path=str(badpath)),base_revision=revision,idempotency_key='blend:'+label+':00001'),project,error=code)
   cancel=root/'cancel';cancel.write_text('cancel');call('cancel-import',dict(operation,base_revision=revision),project,error='cancelled',limits={'cancel_file':str(cancel)})
   call('cancel-export',dict(export,output=str(root/'cancel-export')),project,error='cancelled',limits={'cancel_file':str(cancel)});assert not (root/'cancel-export').exists()
   fresh=root/'budget-project';call('budget-init',{'method':'init','document_id':f'{293:032x}'},fresh);call('transaction-budget',dict(operation,max_added_bytes=1),fresh,error='budget');assert call('budget-unchanged',{'method':'inspect'},fresh)['revision']==empty['revision']
   call('input-budget',operation,fresh,error='budget',limits={'max_input_bytes':1})
   call('source-output-budget',dict(export,output=str(root/'source-output-budget')),project,error='budget',limits={'max_output_bytes':1});assert not any((root/'source-output-budget').rglob('original.blend'))
   call('missing-source',dict(export,output=str(root/'missing-source'),content={'format':'source','asset':'sha256:missing'}),project,error='reference');assert not any((root/'missing-source').rglob('original.blend'))
   baseline=os.environ.get('RENDER_BLEND_BASELINE_HOST')
   if baseline:
    old=call('old-client-journal',{'method':'restore','source':str(archive/'document/document.json')},root/'old-client-project',error='encoding',binary=baseline);assert 'put_source' in old['message']
    checkpoint=json.loads((archive/'document/document.json').read_text());checkpoint['journal']=[];checkpoint['accepted']={};checkpoint_path=root/'snapshot16-checkpoint.json';checkpoint_path.write_text(json.dumps(checkpoint)+'\n')
    call('new-client-checkpoint',{'method':'restore','source':str(checkpoint_path)},root/'checkpoint-project');assert call('checkpoint-state',{'method':'inspect'},root/'checkpoint-project')['revision']==revision
    call('old-client-version',{'method':'restore','source':str(checkpoint_path)},root/'old-client-checkpoint',error='schema_version',binary=baseline)
    assert not (root/'old-client-project').exists() and not (root/'old-client-checkpoint').exists()
   # Native edits remain native; exact source export must still return the original.
   entity=imported['import']['entities']['Quad'];edit={'version':0,'base_revision':revision,'idempotency_key':'blend:rename:00001','max_added_bytes':1024,'commands':[{'operation':'rename','entity':entity,'name':'Edited Quad'}]}
   call('native-edit',{'method':'apply','request':edit},restored);changed=call('native-edited-state',{'method':'inspect'},restored);call('source-after-edit',dict(export,revision=changed['revision'],output=str(root/'source-after-edit')),restored);assert (root/'source-after-edit/source/original.blend').read_bytes()==original
  report['variants'].append({'name':name,'source':source,'external_reference':external,'settings':s,'revision':revision,'report':imported['import'],'empty_revision':empty['revision'],'archive':str(archive/'document/document.json'),'cpu_gpu_rmse':rmse,'native_mesh':mesh,'native_entities':snap['entities'],'native_settings':snap['render_settings']});save()
 report['status']='passed';save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
