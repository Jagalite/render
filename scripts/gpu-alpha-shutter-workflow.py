"""CLI alpha traversal failures through nominal, temporal and partial sequence publication."""
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root=Path(sys.argv[1]).resolve(); root.mkdir(parents=True,exist_ok=False)
fixture=Path('fixtures/alpha-gltf/data').resolve()
model=json.loads(sys.argv[2]) if len(sys.argv)>2 else {'kind':'principled'}
report={'status':'in_progress','calls':[],'variants':[],'model':model}
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
settings.update(width=2,height=2,samples=1,max_depth=1,environment=[0.125,0.25,0.5])
settings['light']['intensity']=[0,0,0]
settings['camera'].update(position=[0,0,3],target=[0,0,0],lens={'kind':'orthographic','xmag':1,'ymag':1,'near':0.1,'far':8})
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
identifier=lambda n:f'{n:032x}'
rational=lambda n,d=1:{'numerator':n,'denominator':d}
project=root/'project';clip=identifier(30002)
try:
    call('init',{'method':'init','document_id':identifier(10060)},project)
    empty=inspect('empty',project)
    call('import',{'method':'import','base_revision':empty['revision'],'idempotency_key':'alpha:shutter:import','max_added_bytes':8388608,'source':{'format':'glb','path':str(Path('fixtures/named-uv/data/roles.glb').resolve()),'policy':{'allow_approximations':True}},'settings':settings},project)
    state=inspect('imported',project);snapshot=state['snapshot'];original=next(e for e in snapshot['entities'] if e['mesh'] is not None)
    material=copy.deepcopy(snapshot['materials'][original['material']]);material['pbr']['advanced']={'model':model,'opacity':{'kind':'mask','factor':0,'cutoff':0.5}}
    commands=[{'operation':'put_material','material':material}];leaves=[original]
    for i in range(1,65):
        entity=copy.deepcopy(original);entity.update(id=identifier(20000+i),parent=None)
        entity['transform']={'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,-i*0.03]],'operations':[]}
        leaves.append(entity);commands.append({'operation':'create_entity','entity':entity})
    tracks=[{'id':identifier(40000+i),'target':{'kind':'entity','entity':e['id']},'property':{'kind':'translation'},'interpolation':'step','keys':[{'time':rational(t),'value':{'kind':'vector','value':[x,0,0]},'incoming':None,'outgoing':None} for t,x in [(0,0),(1,10),(2,0)]]} for i,e in enumerate(leaves)]
    animation={'clips':{clip:{'id':clip,'start':rational(0),'end':rational(2),'extrapolation':'clamp','remap':{'rate':rational(1),'offset':rational(0)},'tracks':tracks}},'rigs':{},'skins':{},'morphs':{}}
    commands.append({'operation':'set_animation','animation':animation})
    call('author-stack',{'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'alpha:shutter:stack01','max_added_bytes':8388608,'commands':commands}},project)
    state=inspect('authored',project);revision=state['revision'];report.update(revision=revision,clip=clip)
    nominal={'revision':revision,'clip':clip,'time':rational(0),'shutter':{'open':rational(1),'close':rational(2),'samples':2}}
    temporal={'revision':revision,'clip':clip,'time':rational(1),'shutter':{'open':rational(-1),'close':rational(1),'samples':2}}
    instant=dict(temporal,shutter={'open':rational(0),'close':rational(0),'samples':1})
    for backend in ['cpu','gpu']:
        for name,request in [('nominal-failure',nominal),('temporal-failure',temporal)]:
            output=root/(backend+'-'+name);call(backend+'-'+name,{'method':'frame','backend':backend,'request':request,'output':str(output)},project,error='budget')
            manifest=json.loads((output/'manifest.json').read_text());assert manifest['status']=='failed' and manifest['bundles']==[] and manifest['artifact_bytes']==0
        call(backend+'-valid',{'method':'frame','backend':backend,'request':instant,'output':str(root/(backend+'-valid'))},project,resource=True)
        seq={k:instant[k] for k in ['revision','clip','shutter']};seq['times']=[rational(1),rational(2)]
        output=root/(backend+'-partial');call(backend+'-partial',{'method':'sequence','backend':backend,'request':seq,'output':str(output)},project,error='budget')
        manifest=json.loads((output/'manifest.json').read_text());assert manifest['status']=='failed' and len(manifest['bundles'])==1 and not list(output.glob('*.pending'))
    call('archive',{'method':'export','revision':revision,'output':str(root/'archive'),'content':{'format':'document'}},project)
    call('restore',{'method':'restore','source':str(root/'archive/document/document.json')},root/'restored')
    call('restored-valid',{'method':'frame','backend':'gpu','request':instant,'output':str(root/'restored-valid')},root/'restored')
    assert (root/'restored-valid/frame/image.pfm').read_bytes()==(root/'gpu-valid/frame/image.pfm').read_bytes()
    call('stale',{'method':'frame','backend':'gpu','request':dict(instant,revision=empty['revision']),'output':str(root/'stale')},project,error='stale_revision')
    assert inspect('unchanged',project)['document_digest']==state['document_digest']
    report.update(status='passed',nominal_request=nominal,temporal_request=temporal,valid_request=instant,sequence_request=seq);save()
except BaseException as error:
    report.update(status='failed',error=str(error));save();raise
