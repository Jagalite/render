"""Native HAIR import, CPU/Metal surface parity, failure atomicity and recovery."""
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
p={'byte_order':'little_endian','meters_per_unit':1,'thickness':'diameter','color_space':'linear_srgb','transparency':'coverage_one_minus_transparency','representation':'swept_polyline_surface','roughness':.6,'tessellation':{'chord_error':.001,'radial_error':.01,'max_samples':8192,'max_vertices':65536,'max_depth':18}}
s=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];s.update(width=16,height=16,samples=16,max_depth=2,environment=[.1,.1,.1]);s['light'].update(position=[1,2,3],intensity=[4,4,4]);s['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':1,'ymag':1,'near':.01,'far':20})
cases=[('default-le','default-le'),('default-be','default-be'),('arrays-le','arrays-le'),('arrays-be','arrays-be'),('srgb','default-le'),('scaled-radius','default-le')]
try:
 for name,file in cases:
  policy=copy.deepcopy(p);settings=copy.deepcopy(s)
  if file.endswith('-be'):policy['byte_order']='big_endian'
  if name=='srgb':policy['color_space']='srgb'
  if name=='scaled-radius':policy.update(meters_per_unit=2,thickness='radius');settings['camera'].update(position=[0,0,5],lens={'kind':'orthographic','xmag':2,'ymag':2,'near':.01,'far':20})
  source={'format':'hair','path':str(Path('fixtures/hair-import/data',file+'.hair').resolve()),'policy':policy}
  project=root/(name+'-project');call(name+'-init',{'method':'init','document_id':f'{10200:032x}'},project);empty=call(name+'-empty',{'method':'inspect'},project)
  op={'method':'import','base_revision':empty['revision'],'idempotency_key':'hair:import:0001','max_added_bytes':8388608,'source':source,'settings':settings}
  imported=call(name+'-import',op,project,resource=True);assert imported['import']['source_strands']==2 and imported['import']['material_groups']==(2 if file.startswith('arrays') else 1)
  assert call(name+'-retry',op,project)==imported
  state=call(name+'-inspect',{'method':'inspect'},project);revision=state['revision'];render={'method':'render','revision':revision}
  evaluated=call(name+'-evaluate',{'method':'evaluate','revision':revision},project);assert sum(c['derived_bytes'] for c in evaluated['conversions'])==imported['import']['observed_derived_json_bytes'];assert sum(c['vertices'] for c in evaluated['conversions'])==imported['import']['derived_vertices'];assert sum(c['triangles'] for c in evaluated['conversions'])==imported['import']['derived_triangles']
  for backend in ['cpu','gpu']:call(name+'-'+backend,dict(render,backend=backend,output=str(root/(name+'-'+backend))),project,resource=True)
  cpu=pfm(root/(name+'-cpu/image/image.pfm'));gpu=pfm(root/(name+'-gpu/image/image.pfm'));rmse=(sum((a-b)**2 for a,b in zip(cpu,gpu,strict=True))/len(cpu))**.5;assert rmse<.002,(name,rmse)
  passes=json.loads((root/(name+'-cpu/image/passes.json')).read_text());gp=json.loads((root/(name+'-gpu/image/passes.json')).read_text());assert passes[4]==gp[4] and any(x is not None for x in passes[4]);assert any(abs(x-.1)>.005 for x in cpu)
  archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
  restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored)
  for backend in ['cpu','gpu']:
   call(name+'-recovered-'+backend,dict(render,backend=backend,output=str(root/(name+'-recovered-'+backend))),restored);assert (root/(name+'-'+backend+'/image/image.pfm')).read_bytes()==(root/(name+'-recovered-'+backend+'/image/image.pfm')).read_bytes()
  call(name+'-stale',dict(op,idempotency_key='hair:stale:00001'),project,error='stale_revision')
  if name=='default-le':
   for label,mutate,error in [('zero-radius',lambda b:b.__setitem__(slice(20,24),struct.pack('<f',0)),'curve'),('flags',lambda b:b.__setitem__(12,34),'unsupported_hair'),('trailing',lambda b:b.append(0),'hair')]:
    b=bytearray(Path(source['path']).read_bytes());mutate(b);bad=root/(label+'.hair');bad.write_bytes(b);call(label,dict(op,base_revision=revision,idempotency_key='hair:'+label+':00001',source=dict(source,path=str(bad))),project,error=error)
   bad_policy=copy.deepcopy(policy);bad_policy['tessellation']['max_vertices']=6;call('sweep-budget',dict(op,base_revision=revision,source=dict(source,policy=bad_policy)),project,error='budget')
   cancel=root/'cancel';cancel.write_text('cancel');call('cancel-import',dict(op,base_revision=revision),project,error='cancelled',limits={'cancel_file':str(cancel)})
   call('input-budget',dict(op,base_revision=revision),project,error='budget',limits={'max_input_bytes':1})
   call('cancel-render',dict(render,backend='gpu',output=str(root/'cancel-render')),project,error='cancelled',limits={'cancel_file':str(cancel)})
   fresh=root/'budget-project';call('budget-init',{'method':'init','document_id':f'{10200:032x}'},fresh);call('transaction-budget',dict(op,max_added_bytes=1),fresh,error='budget');assert call('budget-unchanged',{'method':'inspect'},fresh)['revision']==empty['revision']
  assert call(name+'-unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
  report['variants'].append({'name':name,'source':source,'settings':settings,'revision':revision,'report':imported['import'],'gpu_linear_rmse':rmse,'archive':str(archive/'document/document.json'),'empty_revision':empty['revision']});save()
 for a,b in [('default-le','default-be'),('arrays-le','arrays-be')]:
  for backend in ['cpu','gpu']:assert (root/(a+'-'+backend+'/image/image.pfm')).read_bytes()==(root/(b+'-'+backend+'/image/image.pfm')).read_bytes(),(a,b,backend)
 report['status']='passed';save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
