"""CLI UV authoring, immutable sources, durable recovery and CPU/Metal acceptance."""
import json
import hashlib
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
fixture=Path('fixtures/uv-authoring/data').resolve()
report={'status':'in_progress','calls':[],'variants':[]}
settings=json.loads((fixture/'create.json').read_text())[-1]['settings']
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
    target=root/'project';call('init',{'method':'init','document_id':f'{950:032x}'},target)
    empty=inspect('empty',target)
    create={'version':0,'base_revision':empty['revision'],'idempotency_key':'uv-workflow-create','max_added_bytes':4*1024*1024,'commands':json.loads((fixture/'create.json').read_text())}
    call('create',{'method':'apply','request':create},target)
    state=inspect('created',target);entity=f'{1:032x}';attribute=f'{500:032x}'
    source=next(e for e in state['snapshot']['entities'] if e['id']==entity)['mesh']
    mesh=state['snapshot']['meshes'][source]
    call('source-render',{'method':'render','revision':state['revision'],'backend':'cpu','output':str(root/'source-cpu')},target)
    settings_unwrap=json.loads((fixture/'unwrap.json').read_text());assert sorted(mesh['edge_ids'])==settings_unwrap['seams']
    request={'version':0,'base_revision':state['revision'],'idempotency_key':'uv-workflow-unwrap','entity':entity,'source_mesh':source,'source_uv_asset':None,'operation':{'kind':'unwrap','settings':settings_unwrap},'budget':{'max_output_bytes':4*1024*1024,'max_solver_work':16777216},'max_added_bytes':4*1024*1024}
    operations=[]
    for kind in ['unwrap','pack','second-set']:
        if kind!='unwrap':
            state=inspect('before-'+kind,target)
            request=dict(request,base_revision=state['revision'],idempotency_key='uv-workflow-'+kind,source_mesh=result['report']['output_mesh'],source_uv_asset=result['uv_asset'])
            if kind=='pack':request['operation']={'kind':'pack','attribute':attribute,'settings':json.loads((fixture/'pack.json').read_text())}
            else:
                new=dict(settings_unwrap,attribute='AnotherUV',attribute_id=f'{501:032x}')
                request['operation']={'kind':'unwrap','settings':new}
        if kind=='pack':
            no_fit=dict(request,idempotency_key='uv-workflow-pack-no-fit',operation={'kind':'pack','attribute':attribute,'settings':json.loads((fixture/'padding2-no-fit.json').read_text())})
            call('packing-no-fit',{'method':'author_uv','request':no_fit},target,error='uv_atlas')
        operation={'method':'author_uv','request':request}
        result=call(kind,operation,target,resource=True)
        assert call(kind+'-retry',operation,target)==result
        assert len(result['report']['charts'])==6
        assert all(abs(c['surface_area_square_meters']-1)<1e-12 for c in result['report']['charts'])
        operations.append({'operation':operation,'result':result})
    state=inspect('authored',target);revision=state['revision'];asset=state['snapshot']['uv_assets'][result['uv_asset']]
    assert len(asset['sets'])==2 and asset['sets'][attribute]['atlas']['pixels_per_meter']==12
    assert state['snapshot']['meshes'][source]==mesh
    assert state['snapshot']['version']==17
    assert operations[1]['result']['report']['packing']['atlas_pixels']==4096
    assert operations[1]['result']['report']['packing']['allocated_pixels']<=4096
    call('first-retry-after-two-edits',operations[0]['operation'],target)
    stale=dict(request,idempotency_key='uv-workflow-stale');call('stale',{'method':'author_uv','request':stale},target,error='stale_revision')
    fresh=dict(request,base_revision=revision,idempotency_key='uv-workflow-selection')
    call('stale-selection',{'method':'author_uv','request':fresh},target,error='stale_selection')
    fresh.update(source_mesh=result['report']['output_mesh'],source_uv_asset=result['uv_asset'])
    cancel=root/'cancel';cancel.write_text('cancel')
    call('cancelled',{'method':'author_uv','request':fresh},target,error='cancelled',limits={'cancel_file':str(cancel)})
    call('output-budget',{'method':'author_uv','request':dict(fresh,budget={'max_output_bytes':1,'max_solver_work':16777216})},target,error='budget')
    call('mutation-budget',{'method':'author_uv','request':dict(fresh,max_added_bytes=0,operation={'kind':'unwrap','settings':dict(settings_unwrap,attribute='ThirdUV',attribute_id=f'{502:032x}')})},target,error='budget')
    assert inspect('failures-atomic',target)['document_digest']==state['document_digest']
    render={'method':'render','revision':revision,'backend':'cpu','output':str(root/'box-cpu')}
    call('render-cpu',render,target,resource=True)
    call('render-gpu',dict(render,backend='gpu',output=str(root/'box-gpu')),target,resource=True)
    pixels=pfm(root/'box-cpu/image/image.pfm');gpu=pfm(root/'box-gpu/image/image.pfm')
    source_pixels=pfm(root/'source-cpu/image/image.pfm')
    texture_change=max(abs(a-b) for a,b in zip(source_pixels,pixels,strict=True));assert texture_change>.005,'authored UVs must change checker sampling'
    rmse=(sum((a-b)**2 for a,b in zip(pixels,gpu,strict=True))/len(pixels))**.5;assert rmse<.002,rmse
    call('export',{'method':'export','revision':revision,'output':str(root/'archive'),'content':{'format':'document'}},target)
    baseline=os.environ.get('RENDER_UV_BASELINE_HOST')
    if not baseline:raise ValueError('Acceptance requires RENDER_UV_BASELINE_HOST')
    old_request=root/'old-client.request.json';old_request.write_text(json.dumps({'version':0,'operation':{'method':'restore','source':str(root/'archive/document/document.json')}}))
    old=subprocess.run([baseline,'project',str(root/'old-client'),str(old_request)],capture_output=True,text=True)
    (root/'old-client.stderr.json').write_text(old.stderr);assert old.returncode!=0 and not old.stdout
    assert json.loads(old.stderr)['code']=='encoding' and 'put_uv_asset' in json.loads(old.stderr)['message'],old.stderr
    assert not (root/'old-client').exists()
    checkpoint=json.loads((root/'archive/document/document.json').read_text());checkpoint.update(journal=[],accepted={})
    checkpoint_path=root/'version17-checkpoint.json';checkpoint_path.write_text(json.dumps(checkpoint))
    version_request=root/'old-version.request.json';version_request.write_text(json.dumps({'version':0,'operation':{'method':'restore','source':str(checkpoint_path)}}))
    version=subprocess.run([baseline,'project',str(root/'old-version'),str(version_request)],capture_output=True,text=True)
    (root/'old-version.stderr.json').write_text(version.stderr)
    assert version.returncode!=0 and not version.stdout and json.loads(version.stderr)['code']=='schema_version'
    assert not (root/'old-version').exists()
    report['old_client']={'sha256':hashlib.sha256(Path(baseline).read_bytes()).hexdigest(),'journal_error':'encoding','checkpoint_error':'schema_version','state_not_created':True}
    restored=root/'restored';call('restore',{'method':'restore','source':str(root/'archive/document/document.json')},restored)
    assert inspect('restored',restored)['snapshot']==state['snapshot']
    call('restored-render',dict(render,output=str(root/'box-restored')),restored)
    assert (root/'box-cpu/image/image.pfm').read_bytes()==(root/'box-restored/image/image.pfm').read_bytes()
    report.update(status='passed',create=create,operations=operations,source_mesh=source,revision=revision,snapshot=state['snapshot'],cpu_metal_rmse=rmse,checker_sampling_max_change=texture_change,source_unchanged=True,restored_pixels_identical=True)
    save()
except BaseException as e:
    report.update(status='failed',error=str(e));save();raise
