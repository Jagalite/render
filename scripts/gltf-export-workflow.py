"""CLI evaluated PBR GLB export, independent roundtrip and durable recovery."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
recipe=json.loads(Path('fixtures/gltf-export/cases.json').read_text())
report={'status':'in_progress','calls':[],'variants':[]}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
settings.update(width=32,height=16,samples=8,max_depth=2,environment=[0.1,0.1,0.1])
settings['light']['intensity']=[2,2,2]
settings['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':2.5,'ymag':2.5,'near':0.1,'far':8})
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

def verify_manifest(directory,status='complete'):
    m=json.loads((directory/'manifest.json').read_text());assert m['status']==status,m
    for bundle in m['bundles']:
        for a in bundle['artifacts']:
            data=(directory/a['path']).read_bytes();assert len(data)==a['bytes'];assert digest(data)==a['digest']
    return m

def digest(b):
    import hashlib
    return 'sha256:'+hashlib.sha256(b).hexdigest()

def compare(first,second,mapping):
    a=pfm(first/'image.pfm');b=pfm(second/'image.pfm');rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5
    x=json.loads((first/'passes.json').read_text());y=json.loads((second/'passes.json').read_text())
    assert rmse<2e-5,(first,second,rmse)
    assert max(abs(a-b) for a,b in zip(x[2],y[2],strict=True))<2e-5
    assert max(abs(a-b) for p,q in zip(x[3],y[3],strict=True) for a,b in zip(p,q,strict=True))<2e-5
    assert [mapping.get(i) if i is not None else None for i in x[4]]==y[4]
    return rmse

try:
    for case in recipe['cases']:
        name=case['name'];target=root/name
        call(name+'-init',{'method':'init','document_id':recipe['document_id']},target)
        empty=inspect(name+'-empty',target)
        imported=call(name+'-import',{'method':'import','base_revision':empty['revision'],'idempotency_key':'gltfexport:import:0001','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(Path(case['source']).resolve()),'policy':{'allow_approximations':True}},'settings':settings},target)
        state=inspect(name+'-state',target);revision=state['revision']
        at={'clip':imported['import']['source_clips']['0'],'time':case['time']} if 'time' in case else None
        output=root/(name+'-export');content={'format':'glb','at':at,'policy':recipe['policy']}
        export_op={'method':'export','revision':revision,'output':str(output),'content':content}
        call(name+'-export',export_op,target,resource=True);verify_manifest(output)
        source=output/'scene/scene.glb';export_report=json.loads((output/'scene/report.json').read_text());assert digest(source.read_bytes())==export_report['output_digest']
        call(name+'-repeat',dict(export_op,output=str(root/(name+'-repeat'))),target);assert (root/(name+'-repeat/scene/scene.glb')).read_bytes()==source.read_bytes()
        recovered=root/(name+'-round');call(name+'-round-init',{'method':'init','document_id':recipe['document_id']},recovered)
        reimported=call(name+'-reimport',{'method':'import','base_revision':inspect(name+'-round-empty',recovered)['revision'],'idempotency_key':'gltfexport:reimport:01','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(source),'policy':{'allow_approximations':True}},'settings':settings},recovered,resource=True)
        round_state=inspect(name+'-round-state',recovered)
        mapping={}
        for old,node in export_report['source_entities'].items():
            parent=reimported['import']['source_nodes'][str(node)]
            leaves=[e['id'] for e in round_state['snapshot']['entities'] if e['parent']==parent and e['mesh'] is not None];assert len(leaves)==1;mapping[old]=leaves[0]
        # Export preserves encoded images referenced by evaluated materials. The
        # importer also stores unused source images; those are not GLB dependencies.
        snapshot=state['snapshot'];used=set()
        for entity in snapshot['entities']:
            if entity['mesh'] is None or entity['material'] is None:continue
            pbr=snapshot['materials'][entity['material']].get('pbr') or {}
            for role in ['base_color','metallic_roughness','normal','emission','occlusion']:
                if pbr.get(role):used.add(pbr[role]['image'])
        assert {key:snapshot['images'][key] for key in used}==round_state['snapshot']['images']
        comparisons=[]
        for backend in ['cpu','gpu'] if case['gpu'] else ['cpu']:
            first=root/(name+'-source-'+backend);second=root/(name+'-round-'+backend)
            if at:
                zero={'numerator':0,'denominator':1};request=dict(at,revision=revision,shutter={'open':zero,'close':zero,'samples':1})
                draw={'method':'frame','request':request,'backend':backend,'output':str(first)};bundle='frame'
            else:draw={'method':'render','revision':revision,'backend':backend,'output':str(first)};bundle='image'
            call(name+'-source-'+backend,draw,target,resource=True)
            call(name+'-round-'+backend,{'method':'render','revision':round_state['revision'],'backend':backend,'output':str(second)},recovered,resource=True)
            verify_manifest(first);verify_manifest(second);rmse=compare(first/bundle,second/'image',mapping)
            comparisons.append({'backend':backend,'roundtrip_linear_rmse':rmse,'object_mapping':mapping})
        if not case['gpu']:
            call(name+'-gpu-rejected',{'method':'render','revision':round_state['revision'],'backend':'gpu','output':str(root/(name+'-gpu-rejected'))},recovered,error='unsupported_profile')
        archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':round_state['revision'],'output':str(archive),'content':{'format':'document'}},recovered)
        restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':str(archive/'document/document.json')},restored)
        call(name+'-restored-render',{'method':'render','revision':round_state['revision'],'backend':'cpu','output':str(root/(name+'-restored-image'))},restored)
        assert (root/(name+'-restored-image/image/image.pfm')).read_bytes()==(root/(name+'-round-cpu/image/image.pfm')).read_bytes()
        call(name+'-stale-export',dict(export_op,revision=empty['revision'],output=str(root/(name+'-stale-export'))),target,error='stale_revision');assert not (root/(name+'-stale-export')).exists()
        assert inspect(name+'-unchanged',target)['document_digest']==state['document_digest']
        report['variants'].append(dict(case,revision=revision,at=at,round_revision=round_state['revision'],glb_digest=digest(source.read_bytes()),glb_bytes=source.stat().st_size,comparisons=comparisons,restored_pixels_identical=True));save()
    # Output and core export budgets are separate; neither may publish a scene bundle.
    target=root/'color';state=inspect('negative-base',target)
    for name,policy,limits,error in [
        ('core-budget',dict(recipe['policy'],max_bytes=64),None,'budget'),
        ('vertex-budget',dict(recipe['policy'],max_vertices=3),None,'budget'),
        ('output-budget',recipe['policy'],{'max_output_bytes':64},'budget'),
        ('consent',dict(recipe['policy'],allow_approximations=False),None,'unsupported_gltf')]:
        directory=root/name;call(name,{'method':'export','revision':state['revision'],'output':str(directory),'content':{'format':'glb','policy':policy}},target,limits=limits,error=error)
        m=verify_manifest(directory,'failed');assert not m['bundles'] and not (directory/'scene').exists()
    cancel=root/'cancel';cancel.write_text('cancel');directory=root/'cancelled'
    call('cancelled',{'method':'export','revision':state['revision'],'output':str(directory),'content':{'format':'glb','policy':recipe['policy']}},target,limits={'cancel_file':str(cancel)},error='cancelled');assert not directory.exists()
    assert inspect('negative-unchanged',target)['document_digest']==state['document_digest']
    report['status']='passed';save()
except BaseException as e:
    report['status']='failed';report['error']=str(e);save();raise
