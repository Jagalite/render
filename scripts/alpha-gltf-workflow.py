"""Independent CLI alpha import, durable recovery and analytic rendering acceptance."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
fixture=Path('fixtures/alpha-gltf/data').resolve()
report={'status':'in_progress','calls':[],'variants':[]}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
settings.update(width=32,height=16,samples=64,max_depth=1,environment=[0,0,0])
settings['light']['intensity']=[0,0,0]
settings['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':1,'ymag':1,'near':0.1,'far':5})
report['settings']=settings
def save(): (root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name,op,target,error=None,limits=None,resource=False):
    request={'version':0,'operation':op}
    if limits is not None: request['limits']=limits
    p=root/(f'{len(report["calls"]):03}-'+name+'.request.json');p.write_text(json.dumps(request,indent=2)+'\n')
    command=['target/debug/render-host','project',str(target),str(p)]
    if resource and sys.platform=='darwin':command=['/usr/bin/time','-l']+command
    at=time.monotonic();r=subprocess.run(command,capture_output=True,text=True,timeout=90)
    p.with_suffix('.stdout.json').write_text(r.stdout);p.with_suffix('.stderr.txt').write_text(r.stderr)
    row={'name':name,'command':command,'exit_code':r.returncode,'expected_error':error,'seconds':time.monotonic()-at}
    if resource and r.returncode==0 and sys.platform=='darwin':
        row['resources']={label:int(re.search(r'(\d+)\s+'+label,r.stderr).group(1)) for label in ['maximum resident set size','peak memory footprint']}
    report['calls'].append(row);save()
    if error:
        assert r.returncode!=0 and not r.stdout,(name,r)
        value=json.loads(r.stderr);assert value['code']==error,(name,value)
    else:
        assert r.returncode==0,(name,r.stderr);value=json.loads(r.stdout)
    print(name+': passed',flush=True);return value
def inspect(name,target):return call(name,{'method':'inspect'},target)
def pfm(path):
    h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0'
    v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
try:
    for variant in ['mask','blend','opaque']:
        target=root/variant;call(variant+'-init',{'method':'init','document_id':f'{9800:032x}'},target)
        op={'method':'import','base_revision':inspect(variant+'-empty',target)['revision'],'idempotency_key':'alpha:import:0001','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(fixture/(variant+'.glb')),'policy':{'allow_approximations':True}},'settings':settings}
        imported=call(variant+'-import',op,target,resource=True);assert call(variant+'-retry',op,target)==imported
        state=inspect(variant+'-state',target);revision=state['revision']
        render={'method':'render','revision':revision,'backend':'cpu','output':str(root/(variant+'-cpu'))}
        call(variant+'-render',render,target,resource=True)
        values=pfm(root/(variant+'-cpu/image/image.pfm'))
        means=[]
        for stripe in range(4):
            green=[values[i*3+1] for i in range(512) if (i%32)//8==stripe]
            expected=(1 if stripe>=2 else 0) if variant=='mask' else stripe/6 if variant=='blend' else 1
            mean=sum(green)/len(green);assert abs(mean-expected)<0.03,(variant,stripe,mean,expected);means.append(mean)
        for i in range(512):assert values[3*i]+values[3*i+1]==1 and values[3*i+2]==0
        gpu=dict(render,backend='gpu',output=str(root/(variant+'-gpu')))
        call(variant+'-gpu',gpu,target,resource=True)
        gpu_values=pfm(root/(variant+'-gpu/image/image.pfm'))
        rmse=(sum((a-b)**2 for a,b in zip(values,gpu_values,strict=True))/len(values))**.5
        assert rmse<0.00002,(variant,rmse)
        assert json.loads((root/(variant+'-gpu/manifest.json')).read_text())['status']=='complete'
        call(variant+'-stale',dict(op,idempotency_key='alpha:stale:00001'),target,error='stale_revision')
        call(variant+'-export',{'method':'export','revision':revision,'output':str(root/(variant+'-archive')),'content':{'format':'document'}},target)
        restored=root/(variant+'-restored');call(variant+'-restore',{'method':'restore','source':str(root/(variant+'-archive/document/document.json'))},restored)
        call(variant+'-restored-render',dict(render,output=str(root/(variant+'-restored-image'))),restored)
        assert (root/(variant+'-restored-image/image/image.pfm')).read_bytes()==(root/(variant+'-cpu/image/image.pfm')).read_bytes()
        assert inspect(variant+'-unchanged',target)['document_digest']==state['document_digest']
        report['variants'].append({'name':variant,'revision':revision,'stripe_green_means':means,'restored_pixels_identical':True})
    target=root/'negative';call('negative-init',{'method':'init','document_id':f'{9800:032x}'},target);state=inspect('negative-empty',target)
    broken=json.loads((fixture/'mask.gltf').read_text());broken['materials'][0]['alphaCutoff']=-1
    malformed=root/'malformed.gltf';malformed.write_text(json.dumps(broken))
    op={'method':'import','base_revision':state['revision'],'idempotency_key':'alpha:negative:0001','max_added_bytes':8*1024*1024,'source':{'format':'gltf','path':str(malformed),'buffers':[str(fixture/'mask.bin')],'images':[],'policy':{'allow_approximations':True}},'settings':settings}
    call('malformed',op,target,error='gltf_scene');op['source']['path']=str(fixture/'mask.gltf')
    cancel=root/'cancel';cancel.write_text('cancel')
    call('cancel',op,target,error='cancelled',limits={'cancel_file':str(cancel)})
    call('budget',op,target,error='budget',limits={'max_input_bytes':64})
    assert inspect('negative-unchanged',target)['document_digest']==state['document_digest']
    call('explicit-valid',op,target)
    report['status']='passed';save()
except BaseException as e:
    report['status']='failed';report['error']=str(e);save();raise
