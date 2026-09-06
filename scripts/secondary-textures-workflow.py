"""CLI secondary-only texture filter invariance through typed material edits and recovery."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
fixture=Path('fixtures/secondary-textures/data').resolve()
report={'status':'in_progress','calls':[],'variants':[]}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
settings.update(width=8,height=8,samples=32,max_depth=2,environment=[0,0,0])
settings['light']['intensity']=[0,0,0]
settings['camera'].update(position=[0,0,1],target=[0,0,0],lens={'kind':'orthographic','xmag':0.9,'ymag':0.9,'near':0.01,'far':10})
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
import copy
project=root/'project'
models=[('opaque',None),('principled',{'kind':'principled'}),('coated',{'kind':'coated','weight':0.5,'ior':1.5,'roughness':0.2}),('conductor',{'kind':'conductor','eta':[0.2]*3,'k':[3]*3}),('dielectric',{'kind':'dielectric','ior':1.5})]
def apply(name,commands):
    revision=inspect(name+'-base',project)['revision']
    return call(name,{'method':'apply','request':{'version':0,'base_revision':revision,'idempotency_key':'footprint:'+name,'max_added_bytes':8388608,'commands':commands}},project)
try:
    call('init',{'method':'init','document_id':f'{10070:032x}'},project);empty=inspect('empty',project)
    call('import',{'method':'import','base_revision':empty['revision'],'idempotency_key':'footprint:import:001','max_added_bytes':8388608,'source':{'format':'glb','path':str(fixture/'mask-nearest.glb'),'policy':{'allow_approximations':True}},'settings':settings},project)
    original=inspect('imported',project)['snapshot'];floor=next(m for m in original['materials'].values() if m.get('pbr',{}).get('advanced'));emitter=next(m for m in original['materials'].values() if m.get('pbr',{}).get('emission'))
    for name,model in models:
        row={'name':name,'model':model,'gpu':name in ['opaque','principled'],'filters':[]}
        for filtering,min_filter in [('nearest','nearest'),('mip','linear_mip_linear')]:
            f=copy.deepcopy(floor);e=copy.deepcopy(emitter);f['roughness']=0 if name=='dielectric' else .5;f['pbr']['advanced']={'model':model,'opacity':{'kind':'opaque'}} if model else None;e['pbr']['emission']['sampler']['min']=min_filter
            commands=[{'operation':'put_material','material':m} for m in [f,e]];apply(name+'-'+filtering,commands);state=inspect(name+'-'+filtering+'-state',project);revision=state['revision']
            for backend in ['cpu','gpu'] if row['gpu'] else ['cpu']:
                call(name+'-'+filtering+'-'+backend,{'method':'render','backend':backend,'revision':revision,'output':str(root/(name+'-'+filtering+'-'+backend))},project,resource=True)
            output=root/(name+'-'+filtering+'-archive');call(name+'-'+filtering+'-archive',{'method':'export','revision':revision,'output':str(output),'content':{'format':'document'}},project)
            row['filters'].append({'name':filtering,'revision':revision,'commands':commands,'archive':str(output/'document/document.json')})
        for backend in ['cpu','gpu'] if row['gpu'] else ['cpu']:
            a=root/(name+'-nearest-'+backend)/'image';b=root/(name+'-mip-'+backend)/'image'
            assert (a/'image.pfm').read_bytes()==(b/'image.pfm').read_bytes(),(name,backend,'secondary minification changed pixels')
            assert (a/'passes.json').read_bytes()==(b/'passes.json').read_bytes(),(name,backend,'secondary minification changed passes')
        if row['gpu']:
            a=pfm(root/(name+'-mip-cpu/image/image.pfm'));b=pfm(root/(name+'-mip-gpu/image/image.pfm'));rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5;assert rmse<.002,(name,rmse);row['gpu_linear_rmse']=rmse
        report['variants'].append(row);save()
    state=inspect('final',project);revision=state['revision']
    call('restore',{'method':'restore','source':str(root/'dielectric-mip-archive/document/document.json')},root/'restored')
    call('restored-render',{'method':'render','backend':'cpu','revision':revision,'output':str(root/'restored-image')},root/'restored')
    assert (root/'restored-image/image/image.pfm').read_bytes()==(root/'dielectric-mip-cpu/image/image.pfm').read_bytes()
    call('stale',{'method':'apply','request':{'version':0,'base_revision':empty['revision'],'idempotency_key':'footprint:stale:0001','max_added_bytes':8388608,'commands':commands}},project,error='stale_revision')
    cancel=root/'cancel';cancel.write_text('cancel')
    render={'method':'render','backend':'cpu','revision':revision,'output':str(root/'cancelled')}
    call('cancel',render,project,error='cancelled',limits={'cancel_file':str(cancel)})
    call('budget',dict(render,output=str(root/'budget')),project,error='budget',limits={'max_output_bytes':1})
    assert inspect('unchanged',project)['document_digest']==state['document_digest']
    report['status']='passed';save()
except BaseException as error:
    report.update(status='failed',error=str(error));save();raise
