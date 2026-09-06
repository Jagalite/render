"""CLI tiled painting, UV integration, durable recovery and CPU/Metal acceptance."""
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
native_binary=Path('target/debug/render-host');(root/'tested_native.json').write_text(json.dumps({'path':str(native_binary.resolve()),'sha256':hashlib.sha256(native_binary.read_bytes()).hexdigest(),'bytes':native_binary.stat().st_size,'mtime_ns':native_binary.stat().st_mtime_ns},indent=2)+'\n')
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
def transaction(state,key,commands):
    return dict(version=0,base_revision=state['revision'],idempotency_key=key,max_added_bytes=4*1024*1024,commands=commands)
def step(name,operation):
    result=call(name,operation,target,resource=True)
    report.setdefault('operations',[]).append(dict(operation=json.loads(json.dumps(operation)),result=result));save()
    return result
try:
    target=root/'project';document_id=f'{960:032x}'
    call('init',dict(method='init',document_id=document_id),target);state=inspect('empty',target)
    step('create',dict(method='apply',request=transaction(state,'paint-workflow-create',json.loads((fixture/'create.json').read_text()))))
    state=inspect('created',target);entity=f'{1:032x}';attribute=f'{500:032x}'
    source=next(e for e in state['snapshot']['entities'] if e['id']==entity)['mesh']
    q=dict(version=0,base_revision=state['revision'],idempotency_key='paint-workflow-unwrap',entity=entity,source_mesh=source,source_uv_asset=None,
           operation=dict(kind='unwrap',settings=json.loads((fixture/'unwrap.json').read_text())),budget=dict(max_output_bytes=4*1024*1024,max_solver_work=16777216),max_added_bytes=4*1024*1024)
    result=step('unwrap',dict(method='author_uv',request=q));state=inspect('unwrapped',target)
    q.update(base_revision=state['revision'],idempotency_key='paint-workflow-pack',source_mesh=result['report']['output_mesh'],source_uv_asset=result['uv_asset'],operation=dict(kind='pack',attribute=attribute,settings=json.loads((fixture/'pack.json').read_text())))
    step('pack',dict(method='author_uv',request=q));state=inspect('packed',target)
    call('source-render',dict(method='render',revision=state['revision'],backend='cpu',output=str(root/'source-cpu')),target)
    inputs=json.loads(Path('fixtures/tiled-painting/data/layered-strokes.json').read_text());canvas=inputs['canvas_id'];asset=inputs['canvas']
    # Empty integer-only fixture asset: canonical JSON bytes match the public v0 contract.
    initial_hash='sha256:'+hashlib.sha256(json.dumps(asset,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    step('canvas',dict(method='apply',request=transaction(state,'paint-workflow-canvas',[dict(operation='put_paint_asset',asset=asset),dict(operation='set_paint_canvas',canvas=canvas,source_asset=None,asset=initial_hash)])))
    state=inspect('canvas-created',target);materials=state['snapshot']['materials'];tile_bytes=[];checkpoints=[]
    for i,stroke in enumerate(inputs['strokes']):
        q=dict(version=0,base_revision=state['revision'],idempotency_key=f'paint-workflow-stroke-{i}',canvas=canvas,source_asset=state['snapshot']['paint_canvases'][canvas],stroke=stroke,
               budget=dict(max_dabs=4096,max_pixel_work=16777216,max_touched_tiles=64,max_output_bytes=4*1024*1024),max_added_bytes=4*1024*1024)
        operation=dict(method='author_paint',request=q);result=step(f'stroke-{i}',operation)
        assert call(f'stroke-{i}-retry',operation,target)==result
        state=inspect(f'painted-{i}',target);assert state['snapshot']['materials']==materials
        assert state['snapshot']['paint_assets'][initial_hash]==asset
        checkpoints.append(result['report']['output']);tile_bytes.append(result['report']['copied_tile_bytes'])
    last_stroke=q;revision=state['revision'];before=state['document_digest']
    stale=dict(last_stroke,idempotency_key='paint-workflow-stale');call('stale',dict(method='author_paint',request=stale),target,error='stale_revision')
    stale.update(base_revision=revision);call('stale-source',dict(method='author_paint',request=stale),target,error='stale_selection')
    fresh=dict(last_stroke,base_revision=revision,source_asset=checkpoints[-1],idempotency_key='paint-workflow-rejected',stroke=dict(inputs['strokes'][4],id=f'{920:032x}'))
    call('budget',dict(method='author_paint',request=dict(fresh,budget=dict(fresh['budget'],max_pixel_work=1))),target,error='budget')
    cancel=root/'cancel';cancel.write_text('cancel');call('cancelled',dict(method='author_paint',request=fresh),target,error='cancelled',limits=dict(cancel_file=str(cancel)))
    call('mutation-budget',dict(method='author_paint',request=dict(fresh,max_added_bytes=0)),target,error='budget')
    assert inspect('failures-atomic',target)['document_digest']==before
    step('undo',dict(method='apply',request=transaction(state,'paint-workflow-undo',[dict(operation='set_paint_canvas',canvas=canvas,source_asset=checkpoints[-1],asset=checkpoints[0])])));state=inspect('undone',target)
    assert state['snapshot']['paint_canvases'][canvas]==checkpoints[0]
    step('redo',dict(method='apply',request=transaction(state,'paint-workflow-redo',[dict(operation='set_paint_canvas',canvas=canvas,source_asset=checkpoints[0],asset=checkpoints[-1])])));state=inspect('redone',target)
    q=dict(version=0,base_revision=state['revision'],idempotency_key='paint-workflow-bake',canvas=canvas,source_asset=checkpoints[-1],budget=dict(max_pixel_work=16777216,max_encoded_bytes=4*1024*1024),max_added_bytes=4*1024*1024)
    operation=dict(method='bake_paint',request=q);baked=step('bake',operation);assert call('bake-retry',operation,target)==baked
    state=inspect('baked',target);assert state['snapshot']['materials']==materials
    material=dict(materials[f'{2:032x}']);material.update(texture=None,base_color=[1,1,1],pbr=dict(double_sided=False,base_color=dict(image=baked['report']['image'],role='srgb_color',uv_attribute=attribute,sampler=dict(wrap_s='clamp',wrap_t='clamp',mag='linear',min='linear_mip_linear')),metallic_roughness=None,normal=None,emission=None,occlusion=None,normal_scale=1,occlusion_strength=1))
    step('bind-material',dict(method='apply',request=transaction(state,'paint-workflow-bind-image',[dict(operation='put_material',material=material)])))
    state=inspect('final',target);revision=state['revision'];assert state['snapshot']['version']==18
    call('first-stroke-retry-after-later-edits',report['operations'][4]['operation'],target)
    render=dict(method='render',revision=revision,backend='cpu',output=str(root/'box-cpu'))
    call('render-cpu',render,target,resource=True);call('render-gpu',dict(render,backend='gpu',output=str(root/'box-gpu')),target,resource=True)
    cpu=pfm(root/'box-cpu/image/image.pfm');gpu=pfm(root/'box-gpu/image/image.pfm');original=pfm(root/'source-cpu/image/image.pfm')
    change=max(abs(a-b) for a,b in zip(cpu,original,strict=True));assert change>.005
    rmse=(sum((a-b)**2 for a,b in zip(cpu,gpu,strict=True))/len(cpu))**.5;assert rmse<.002
    call('export',dict(method='export',revision=revision,output=str(root/'archive'),content=dict(format='document')),target)
    baseline=os.environ['RENDER_PAINT_BASELINE_HOST'];archive=root/'archive/document/document.json'
    for label,source,expected in [('journal',archive,'encoding'),('checkpoint',root/'version18-checkpoint.json','schema_version')]:
        if label=='checkpoint':
            checkpoint=json.loads(archive.read_text());checkpoint.update(journal=[],accepted={});source.write_text(json.dumps(checkpoint))
        request=root/f'old-{label}.request.json';request.write_text(json.dumps(dict(version=0,operation=dict(method='restore',source=str(source)))))
        destination=root/f'old-{label}';old=subprocess.run([baseline,'project',str(destination),str(request)],capture_output=True,text=True)
        (root/f'old-{label}.stderr.json').write_text(old.stderr);assert old.returncode!=0 and not old.stdout and json.loads(old.stderr)['code']==expected;assert not destination.exists()
    restored=root/'restored';call('restore',dict(method='restore',source=str(archive)),restored)
    assert inspect('restored',restored)['snapshot']==state['snapshot']
    call('restored-render',dict(render,output=str(root/'box-restored')),restored)
    assert (root/'box-cpu/image/image.pfm').read_bytes()==(root/'box-restored/image/image.pfm').read_bytes()
    report.update(status='passed',document_id=document_id,revision=revision,snapshot=state['snapshot'],cpu_metal_rmse=rmse,paint_sampling_max_change=change,source_unchanged=True,restored_pixels_identical=True,copied_tile_bytes=tile_bytes,old_client=dict(sha256=hashlib.sha256(Path(baseline).read_bytes()).hexdigest(),journal_error='encoding',checkpoint_error='schema_version',state_not_created=True))
    save()
except BaseException as e:
    report.update(status='failed',error=str(e));save();raise
