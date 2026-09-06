"""Sparse media temporal admission and partial sequence publication."""
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
rational=lambda n,d=1:{'numerator':n,'denominator':d}
project=root/'project';clip=identifier(10700)
try:
 call('restore-source',{'method':'restore','source':str(Path(sys.argv[2]).resolve())},project);state=call('source-state',{'method':'inspect'},project);empty=state['revision']
 track={'id':identifier(10701),'target':{'kind':'entity','entity':identifier(10601)},'property':{'kind':'translation'},'interpolation':'step','keys':[{'time':rational(t),'value':{'kind':'vector','value':[x,0,0]},'incoming':None,'outgoing':None} for t,x in [(0,0),(1,100000000.3),(2,0)]]}
 animation={'clips':{clip:{'id':clip,'start':rational(0),'end':rational(2),'extrapolation':'clamp','remap':{'rate':rational(1),'offset':rational(0)},'tracks':[track]}},'rigs':{},'skins':{},'morphs':{}}
 call('author-animation',{'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'media:shutter:author:001','max_added_bytes':8388608,'commands':[{'operation':'set_animation','animation':animation}]}},project)
 state=call('authored',{'method':'inspect'},project);revision=state['revision']
 nominal={'revision':revision,'clip':clip,'time':rational(1),'shutter':{'open':rational(1),'close':rational(2),'samples':2}}
 temporal={'revision':revision,'clip':clip,'time':rational(0),'shutter':{'open':rational(0),'close':rational(2),'samples':2}}
 valid=dict(temporal,shutter={'open':rational(0),'close':rational(1,2),'samples':2})
 for name,request in [('nominal-failure',nominal),('temporal-failure',temporal)]:
  output=root/('gpu-'+name);call(name,{'method':'frame','backend':'gpu','request':request,'output':str(output)},project,error='precision');manifest=json.loads((output/'manifest.json').read_text());assert manifest['status']=='failed' and manifest['bundles']==[] and manifest['artifact_bytes']==0
 for backend in ['cpu','gpu']:call(backend+'-valid',{'method':'frame','backend':backend,'request':valid,'output':str(root/(backend+'-valid'))},project,resource=True)
 cpu=pfm(root/'cpu-valid/frame/image.pfm');gpu=pfm(root/'gpu-valid/frame/image.pfm');assert max(abs(a-b) for a,b in zip(cpu,gpu,strict=True))<2e-5
 seq={'revision':revision,'clip':clip,'shutter':valid['shutter'],'times':[rational(0),rational(1)]};output=root/'gpu-partial';call('gpu-partial',{'method':'sequence','backend':'gpu','request':seq,'output':str(output)},project,error='precision');manifest=json.loads((output/'manifest.json').read_text());assert manifest['status']=='failed' and len(manifest['bundles'])==1 and not list(output.glob('*.pending'))
 complete=dict(seq,times=[rational(0),rational(2)]);call('gpu-complete',{'method':'sequence','backend':'gpu','request':complete,'output':str(root/'gpu-complete')},project)
 call('archive',{'method':'export','revision':revision,'output':str(root/'archive'),'content':{'format':'document'}},project)
 call('stale',{'method':'frame','backend':'gpu','request':dict(valid,revision=empty),'output':str(root/'stale')},project,error='stale_revision')
 assert call('unchanged',{'method':'inspect'},project)['document_digest']==state['document_digest']
 report.update(status='passed',revision=revision,clip=clip,nominal_request=nominal,temporal_request=temporal,valid_request=valid,sequence_request=seq,complete_request=complete,max_absolute_error=max(abs(a-b) for a,b in zip(cpu,gpu,strict=True)));save()
except BaseException as e:report.update(status='failed',error=str(e));save();raise
