"""Native CLI multi-bounce acceptance; all outputs go to a new directory."""
import hashlib,json,re,struct,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
(root/'requests').mkdir();(root/'responses').mkdir();project=root/'project'
report={'status':'in_progress','calls':[],'comparisons':[]}
def save():(root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name,op,target=project,limits=None,error=None):
 request={'version':0,'operation':op}
 if limits:request['limits']=limits
 p=root/'requests'/(name+'.json');p.write_text(json.dumps(request,indent=2)+'\n')
 cmd=[str(Path('target/debug/render-host').resolve()),'project',str(target),str(p)]
 if sys.platform=='darwin' and not error:cmd=['/usr/bin/time','-l']+cmd
 at=time.monotonic();r=subprocess.run(cmd,text=True,capture_output=True,timeout=120)
 (root/'responses'/(name+'.stdout.json')).write_text(r.stdout);(root/'responses'/(name+'.stderr.txt')).write_text(r.stderr)
 row={'name':name,'seconds':time.monotonic()-at,'exit_code':r.returncode,'expected_error':error}
 if sys.platform=='darwin' and not error and r.returncode==0:row['resources']={label:int(re.search(r'(\d+)\s+'+label,r.stderr).group(1)) for label in ['maximum resident set size','peak memory footprint']}
 report['calls'].append(row);save()
 if error:assert r.returncode!=0 and json.loads(r.stderr)['code']==error,(name,r.stderr);return
 assert r.returncode==0,(name,r.stderr)
 return json.loads(r.stdout)
def inspect(name='inspect',target=project):return call(name,{'method':'inspect'},target)
def apply(name,commands,error=None):
 return call(name,{'method':'apply','request':{'version':0,'base_revision':inspect(name+'-base')['revision'],'idempotency_key':'multibounce:'+name,'commands':commands,'max_added_bytes':8388608}},error=error)
def bundle(path):
 m=json.loads((path/'manifest.json').read_text());assert m['status']=='complete'
 for b in m['bundles']:
  for a in b['artifacts']:
   data=(path/a['path']).read_bytes();assert len(data)==a['bytes'] and 'sha256:'+hashlib.sha256(data).hexdigest()==a['digest']
def rgb(path):
 h,d,e,data=path.read_bytes().split(b'\n',3);assert h==b'PF' and e==b'-1.0';w,y=map(int,d.split());return struct.unpack('<'+'f'*(w*y*3),data)
try:
 call('init',{'method':'init','document_id':f'{9500:032x}'})
 commands=json.loads(Path('fixtures/multibounce-pbr/create.json').read_text());apply('author',commands)
 settings=commands[-1]['settings'];means=[];first=None
 for depth in [1,2,4,16]:
  settings['max_depth']=depth;apply('depth-'+str(depth),[{'operation':'set_render_settings','settings':settings}]);state=inspect('state-'+str(depth));revision=state['revision']
  for backend in ['cpu','gpu']:
   output=root/f'{backend}-{depth}';call(f'{backend}-{depth}',{'method':'render','revision':revision,'backend':backend,'output':str(output)});bundle(output)
  a=rgb(root/f'cpu-{depth}/image/image.pfm');b=rgb(root/f'gpu-{depth}/image/image.pfm');rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5
  cp=json.loads((root/f'cpu-{depth}/image/passes.json').read_text());gp=json.loads((root/f'gpu-{depth}/image/passes.json').read_text());assert cp[4]==gp[4];assert rmse<.002,rmse
  if first is None:first=cp[2:]
  assert first==cp[2:],'primary passes changed with depth'
  means.append(sum(a)/len(a));report['comparisons'].append({'depth':depth,'linear_rmse':rmse,'object_mismatches':0,'mean_rgb':means[-1]})
  call('export-'+str(depth),{'method':'export','revision':revision,'content':{'format':'document'},'output':str(root/f'archive-{depth}')});bundle(root/f'archive-{depth}')
 assert means[0]==0 and means[1]>0.05 and means[2]>means[1] and means[3]>means[2],means
 call('restore',{'method':'restore','source':str(root/'archive-4/document/document.json')},root/'restored');restored=inspect('restored-state',root/'restored')
 call('restored-render',{'method':'render','revision':restored['revision'],'backend':'cpu','output':str(root/'restored-cpu')},root/'restored')
 assert (root/'restored-cpu/image/image.pfm').read_bytes()==(root/'cpu-4/image/image.pfm').read_bytes()
 before=inspect('before-negative')['document_digest']
 for depth in [0,17]:
  invalid=dict(settings,max_depth=depth);apply('invalid-depth-'+str(depth),[{'operation':'set_render_settings','settings':invalid}],error='render_settings')
 for backend in ['cpu','gpu']:
  call('stale-'+backend,{'method':'render','revision':'stale','backend':backend,'output':str(root/('stale-'+backend))},error='stale_revision')
  flag=root/'cancel';flag.write_text('cancel')
  call('cancel-'+backend,{'method':'render','revision':revision,'backend':backend,'output':str(root/('cancel-'+backend))},limits={'cancel_file':str(flag)},error='cancelled')
 call('output-budget',{'method':'render','revision':revision,'backend':'cpu','output':str(root/'limited')},limits={'max_output_bytes':32},error='budget')
 assert inspect('unchanged')['document_digest']==before
 report.update(status='passed',source='fixtures/multibounce-pbr/create.json',restored_identical=True,invalid_depths_rejected=True,stale_rejected=True,cancelled=True,authored_state_immutable=True);save();print(json.dumps(report['comparisons']))
except BaseException as e:report.update(status='failed',error=str(e));save();raise
