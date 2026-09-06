"""CLI vertex color import, durable recovery, admission and CPU/Metal acceptance."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
fixture=Path('fixtures/vertex-colors/data').resolve()
report={'status':'in_progress','calls':[],'variants':[]}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
settings.update(width=32,height=16,samples=8,max_depth=2,environment=[0.1,0.1,0.1])
settings['light']['intensity']=[2,2,2]
settings['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':2.5,'ymag':1,'near':0.1,'far':5})
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
    for variant in ['rgba','rgb','u8','u16','sparse']:
        target=root/variant;call(variant+'-init',{'method':'init','document_id':f'{9950:032x}'},target)
        op={'method':'import','base_revision':inspect(variant+'-empty',target)['revision'],'idempotency_key':'vertexcolor:import:0001','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(fixture/(variant+'.glb')),'policy':{'allow_approximations':True}},'settings':settings}
        imported=call(variant+'-import',op,target,resource=True);assert call(variant+'-retry',op,target)==imported
        state=inspect(variant+'-state',target);revision=state['revision']
        render={'method':'render','revision':revision,'backend':'cpu','output':str(root/(variant+'-cpu'))}
        call(variant+'-render',render,target,resource=True)
        values=pfm(root/(variant+'-cpu/image/image.pfm'))
        gpu=dict(render,backend='gpu',output=str(root/(variant+'-gpu')))
        call(variant+'-gpu',gpu,target,resource=True)
        gpu_values=pfm(root/(variant+'-gpu/image/image.pfm'))
        rmse=(sum((a-b)**2 for a,b in zip(values,gpu_values,strict=True))/len(values))**.5
        assert rmse<.002,rmse
        cpu_passes=json.loads((root/(variant+'-cpu/image/passes.json')).read_text());gpu_passes=json.loads((root/(variant+'-gpu/image/passes.json')).read_text())
        assert cpu_passes[4]==gpu_passes[4]
        call(variant+'-stale',dict(op,idempotency_key='vertexcolor:stale:00001'),target,error='stale_revision')
        call(variant+'-export',{'method':'export','revision':revision,'output':str(root/(variant+'-archive')),'content':{'format':'document'}},target)
        restored=root/(variant+'-restored');call(variant+'-restore',{'method':'restore','source':str(root/(variant+'-archive/document/document.json'))},restored)
        call(variant+'-restored-render',dict(render,output=str(root/(variant+'-restored-image'))),restored)
        assert (root/(variant+'-restored-image/image/image.pfm')).read_bytes()==(root/(variant+'-cpu/image/image.pfm')).read_bytes()
        assert inspect(variant+'-unchanged',target)['document_digest']==state['document_digest']
        report['variants'].append({'name':variant,'revision':revision,'cpu_metal_rmse':rmse,'restored_pixels_identical':True})
    target=root/'negative';call('negative-init',{'method':'init','document_id':f'{9950:032x}'},target);state=inspect('negative-empty',target)
    broken=json.loads((fixture/'rgba.gltf').read_text());color=broken['meshes'][0]['primitives'][0]['attributes']['COLOR_0'];broken['accessors'][color]['count']=3
    malformed=root/'malformed.gltf';malformed.write_text(json.dumps(broken))
    op={'method':'import','base_revision':state['revision'],'idempotency_key':'vertexcolor:negative:0001','max_added_bytes':8*1024*1024,'source':{'format':'gltf','path':str(malformed),'buffers':[str(fixture/'rgba.bin')],'images':[],'policy':{'allow_approximations':True}},'settings':settings}
    call('malformed',op,target,error='gltf_scene');op['source']['path']=str(fixture/'rgba.gltf')
    cancel=root/'cancel';cancel.write_text('cancel')
    call('cancel',op,target,error='cancelled',limits={'cancel_file':str(cancel)})
    call('budget',op,target,error='budget',limits={'max_input_bytes':64})
    assert inspect('negative-unchanged',target)['document_digest']==state['document_digest']
    call('explicit-valid',op,target)
    report['status']='passed';save()
except BaseException as e:
    report['status']='failed';report['error']=str(e);save();raise
