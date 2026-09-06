"""CLI secondary-only texture filter invariance through typed material edits and recovery."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
recipe=json.loads(Path('fixtures/gpu-dielectric/cases.json').read_text())
recipe['cases'].extend(json.loads(Path('fixtures/gpu-dielectric/data/analytic-cases.json').read_text())['cases'])
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
import copy,hashlib
rational=lambda n,d=1:{'numerator':n,'denominator':d}
try:
    for case in recipe['cases']:
        name=case['name'];project=root/(name+'-project');source=Path(case['source']).resolve()
        assert hashlib.sha256(source.read_bytes()).hexdigest()==case['source_sha256']
        s=copy.deepcopy(settings)
        s.update(max_depth=4)
        if not case['secondary']:
            s.update(width=16,height=16,samples=16,max_depth=4,environment=[.125,.25,.5])
            s['camera'].update(position=[0,0,3],lens={'kind':'orthographic','xmag':2.5,'ymag':2.5,'near':.1,'far':10})
            s['light'].update(position=[1,2,3],intensity=[1,1,1])
        s.update(case.get('settings',{}));s['camera'].update(case.get('camera',{}))
        call(name+'-init',{'method':'init','document_id':f'{10080:032x}'},project);empty=inspect(name+'-empty',project)
        imported=call(name+'-import',{'method':'import','base_revision':empty['revision'],'idempotency_key':'surfaces:import:001','max_added_bytes':8388608,'source':{'format':'glb','path':str(source),'policy':{'allow_approximations':True}},'settings':s},project)
        state=inspect(name+'-imported',project);commands=[]
        for original in state['snapshot']['materials'].values():
            if original.get('pbr') is None:continue
            if case['glass_only'] and original.get('emission')!=[0,0,0]:continue
            m=copy.deepcopy(original);m.update(roughness=0,metallic=0);p=m['pbr'];p['advanced']={'model':case['model'],'opacity':case.get('opacity',(p.get('advanced') or {}).get('opacity',{'kind':'opaque'}))}
            commands.append({'operation':'put_material','material':m})
        for change in case.get('transforms',[]):
            entity=next(e for e in state['snapshot']['entities'] if e['name']==change['name'])
            commands.append({'operation':'set_transform','entity':entity['id'],'transform':change['transform']})
        apply={'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'surfaces:material:001','max_added_bytes':8388608,'commands':commands}}
        applied=call(name+'-author',apply,project);assert call(name+'-retry',apply,project)==applied
        state=inspect(name+'-authored',project);revision=state['revision'];row=dict(case,revision=revision,settings=s)
        if case['animated']:
            clip=imported['import']['source_clips']['0'];request={'revision':revision,'clip':clip,'time':rational(1,2),'shutter':{'open':rational(-1,32),'close':rational(1,32),'samples':3}}
            op={'method':'frame','request':request};folder='frame';row['request']=request
        else:op={'method':'render','revision':revision};folder='image'
        for backend in ['cpu','gpu']:
            call(name+'-'+backend,dict(op,backend=backend,output=str(root/(name+'-'+backend))),project,resource=True)
        a=pfm(root/(name+'-cpu')/folder/'image.pfm');b=pfm(root/(name+'-gpu')/folder/'image.pfm')
        rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5;assert rmse<.002,(name,rmse);assert 'expected' in case or any(x>.01 for x in a)
        if 'expected' in case:
            assert max(abs(x-y) for x,y in zip(a,case['expected'],strict=True))<2e-6,(name,'CPU analytic',a,case['expected'])
            assert max(abs(x-y) for x,y in zip(b,case['expected'],strict=True))<2e-5,(name,'GPU analytic',b,case['expected'])
        ca=json.loads((root/(name+'-cpu')/folder/'passes.json').read_text());ga=json.loads((root/(name+'-gpu')/folder/'passes.json').read_text());assert ca[4]==ga[4]
        row.update(gpu_linear_rmse=rmse,folder=folder)
        if case['animated']:
            seq={k:request[k] for k in ['revision','clip','shutter']};seq['times']=[rational(1,4),rational(3,4)];row['sequence']=seq
            call(name+'-sequence',{'method':'sequence','backend':'gpu','request':seq,'output':str(root/(name+'-sequence'))},project,resource=True)
            manifest=json.loads((root/(name+'-sequence/manifest.json')).read_text());assert manifest['status']=='complete' and len(manifest['bundles'])==2
        archive=root/(name+'-archive');call(name+'-archive',{'method':'export','revision':revision,'output':str(archive),'content':{'format':'document'}},project)
        row['archive']=str(archive/'document/document.json');restored=root/(name+'-restored');call(name+'-restore',{'method':'restore','source':row['archive']},restored)
        call(name+'-restored-render',dict(op,backend='gpu',output=str(root/(name+'-recovered'))),restored)
        assert (root/(name+'-gpu')/folder/'image.pfm').read_bytes()==(root/(name+'-recovered')/folder/'image.pfm').read_bytes()
        bad=copy.deepcopy(commands[0]);bad['material']['pbr']['advanced']['model']={'kind':'dielectric','ior':4}
        request={'version':0,'base_revision':revision,'idempotency_key':'surfaces:invalid:001','max_added_bytes':8388608,'commands':[bad]}
        call(name+'-invalid',{'method':'apply','request':request},project,error='material')
        call(name+'-stale',{'method':'apply','request':dict(request,base_revision=empty['revision'],idempotency_key='surfaces:stale:001')},project,error='stale_revision')
        cancel=root/'cancel';cancel.write_text('cancel');call(name+'-cancel',dict(op,backend='gpu',output=str(root/(name+'-cancel'))),project,error='cancelled',limits={'cancel_file':str(cancel)})
        call(name+'-budget',dict(op,backend='gpu',output=str(root/(name+'-budget'))),project,error='budget',limits={'max_output_bytes':1})
        assert inspect(name+'-unchanged',project)['document_digest']==state['document_digest']
        report['variants'].append(row);save()
    report['status']='passed';save()
except BaseException as error:
    report.update(status='failed',error=str(error));save();raise
