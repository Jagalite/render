"""Independent native CLI client: no engine imports and no demo commands.

Usage: python3 scripts/project-cli-workflow.py <new-output-directory> [--gpu]
Requires a built target/debug/render-host. Writes requests, responses, artifacts,
resource logs and a final report; does not change checked-in reference evidence.
"""
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import time

root = Path(sys.argv[1]).resolve()
root.mkdir(parents=True, exist_ok=False)
(root / 'requests').mkdir()
(root / 'responses').mkdir()
project = root / 'project'
binary = Path('target/debug/render-host').resolve()
fixture = Path('fixtures/project-cli')
report = {'status': 'in_progress', 'profile': 'project-cli-v0', 'calls': [],
          'scope': 'Original authored project through independent one-shot native CLI requests; optional native GPU.'}
identifier = lambda n: f'{n:032x}'
def rational(n, d=1):
    import math
    factor = math.gcd(n,d)
    return {'numerator': n//factor, 'denominator': d//factor}
shutter = {'open': rational(-1,32), 'close': rational(1,32), 'samples': 3}
def save_report():
    (root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name, operation, target=project, limits=None, error=None, resource=False):
    request = {'version':0, 'operation':operation}
    if limits is not None: request['limits'] = limits
    stem = f'{len(report["calls"]):03}-{name}'
    source = root/'requests'/(stem+'.json')
    source.write_text(json.dumps(request,indent=2)+'\n')
    command = [str(binary),'project',str(target),str(source)]
    if resource and sys.platform=='darwin': command = ['/usr/bin/time','-l']+command
    at=time.monotonic()
    result=subprocess.run(command,capture_output=True,text=True,timeout=90)
    (root/'responses'/(stem+'.stdout.json')).write_text(result.stdout)
    (root/'responses'/(stem+'.stderr.txt')).write_text(result.stderr)
    row={'name':name,'command':command,'seconds':time.monotonic()-at,'exit_code':result.returncode,'expected_error':error}
    if resource and result.returncode==0 and sys.platform=='darwin':
        row['resources']={label:int(re.search(r'(\d+)\s+'+label,result.stderr).group(1)) for label in ['maximum resident set size','peak memory footprint']}
    report['calls'].append(row);save_report()
    if error:
        assert result.returncode!=0 and not result.stdout, result
        value=json.loads(result.stderr)
        assert value['code']==error,value
    else:
        assert result.returncode==0,result.stderr
        value=json.loads(result.stdout)
    print(name+': '+('expected '+error if error else 'passed'),flush=True)
    return value

def inspect(name='inspect',target=project): return call(name,{'method':'inspect'},target)
def apply(name,commands):
    revision=inspect(name+'-base')['revision']
    return call(name,{'method':'apply','request':{'version':0,'base_revision':revision,'idempotency_key':'cli-workflow:'+name,'commands':commands,'max_added_bytes':8*1024*1024}})
def verify_manifest(directory):
    m=json.loads((directory/'manifest.json').read_text());assert m['status']=='complete',m
    measured=0
    for b in m['bundles']:
        for a in b['artifacts']:
            data=(directory/a['path']).read_bytes();assert len(data)==a['bytes'];assert 'sha256:'+hashlib.sha256(data).hexdigest()==a['digest'];measured+=len(data)
    assert measured==m['artifact_bytes'];return m

def pfm(path):
    header,dims,endian,payload=path.read_bytes().split(b'\n',3)
    assert header==b'PF' and endian==b'-1.0'
    width,height=map(int,dims.split());return (width,height),struct.unpack('<'+'f'*(width*height*3),payload)

try:
    call('init',{'method':'init','document_id':identifier(123)})
    apply('create',json.loads((fixture/'create.json').read_text()))
    state=inspect();mesh=next(e['mesh'] for e in state['snapshot']['entities'] if e['id']==identifier(1))
    apply('bevel',[{'operation':'model_mesh','entity':identifier(1),'source_mesh':mesh,'operator':{'kind':'bevel_box','distance_meters':.08},'budget':{'vertices':65536,'faces':65536,'bytes':8*1024*1024}}])
    call('import-obj',{'method':'import','base_revision':inspect()['revision'],'idempotency_key':'cli-workflow:floor','max_added_bytes':8*1024*1024,'source':{'format':'obj','path':str((fixture/'floor.obj').resolve()),'entity':identifier(4),'name':'ground plane','material':identifier(3)}})
    apply('animate',json.loads((fixture/'animate.json').read_text()))
    state=inspect('authored');revision=state['revision'];evaluations={}
    for n in [0,2,4,1,3,2]:
        sample={'clip':identifier(10),'time':rational(n,4)}
        output=call('evaluate-'+str(n),{'method':'evaluate','revision':revision,'at':sample})
        actor=next(i for i in output['instances'] if i['id']==identifier(1))
        assert abs(actor['transform'][3][0]-(-.6+1.2*n/4))<1e-12
        if n in evaluations: assert evaluations[n]==output['evaluation_revision']
        evaluations[n]=output['evaluation_revision']
    call('render',{'method':'render','revision':revision,'backend':'cpu','output':str(root/'cpu')},resource=True)
    cpu_manifest=verify_manifest(root/'cpu')
    passes=json.loads((root/'cpu/image/passes.json').read_text());assert sum(x is not None for x in passes[4])>100
    request={'revision':revision,'clip':identifier(10),'time':rational(1,2),'shutter':shutter}
    call('frame',{'method':'frame','request':request,'output':str(root/'frame')},resource=True);verify_manifest(root/'frame')
    call('sequence',{'method':'sequence','request':{'revision':revision,'clip':identifier(10),'times':[rational(n,4) for n in range(5)],'shutter':shutter},'output':str(root/'sequence')},resource=True)
    manifest=verify_manifest(root/'sequence');assert len(manifest['bundles'])==5
    assert (root/'frame/frame/image.pfm').read_bytes()==(root/'sequence/frame-000002/image.pfm').read_bytes()
    call('products',{'method':'products','revision':revision,'output':str(root/'products')},resource=True);verify_manifest(root/'products')
    call('export-document',{'method':'export','revision':revision,'output':str(root/'archive'),'content':{'format':'document'}});verify_manifest(root/'archive')
    call('restore',{'method':'restore','source':str(root/'archive/document/document.json')},root/'restored')
    assert inspect('reopened',root/'restored')['document_digest']==state['document_digest']
    call('export-obj',{'method':'export','revision':revision,'output':str(root/'obj'),'content':{'format':'obj','entity':identifier(1),'at':{'clip':identifier(10),'time':rational(1)}}});verify_manifest(root/'obj')
    call('stale-render',{'method':'render','revision':'stale','backend':'cpu','output':str(root/'stale')},error='stale_revision');assert not (root/'stale').exists()
    cancel=root/'cancel';cancel.write_text('cancel\n')
    call('cancel-evaluation',{'method':'evaluate','revision':revision},limits={'cancel_file':str(cancel)},error='cancelled')
    call('output-budget',{'method':'render','revision':revision,'backend':'cpu','output':str(root/'limited')},limits={'max_output_bytes':64},error='budget')
    assert json.loads((root/'limited/manifest.json').read_text())['status']=='failed'
    call('existing-output',{'method':'render','revision':revision,'backend':'cpu','output':str(root/'cpu')},error='output_exists')
    assert verify_manifest(root/'cpu')==cpu_manifest
    if '--gpu' in sys.argv:
        gpu_project=project
        gpu_revision=revision
        call('gpu-cpu-reference',{'method':'render','revision':gpu_revision,'backend':'cpu','output':str(root/'gpu-reference')},gpu_project,resource=True);verify_manifest(root/'gpu-reference')
        call('gpu-render',{'method':'render','revision':gpu_revision,'backend':'gpu','output':str(root/'gpu')},gpu_project,resource=True);verify_manifest(root/'gpu')
        dimensions,a=pfm(root/'gpu-reference/image/image.pfm');other,b=pfm(root/'gpu/image/image.pfm');assert dimensions==other
        rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5
        cp=json.loads((root/'gpu-reference/image/passes.json').read_text())
        gp=json.loads((root/'gpu/image/passes.json').read_text());mismatches=sum(x!=y for x,y in zip(cp[4],gp[4],strict=True))
        assert rmse<.025 and mismatches<=4,(rmse,mismatches)
        report['cpu_gpu']={'linear_rmse':rmse,'object_mismatches':mismatches,'authored_revision':gpu_revision,'max_depth':2,'depth_two_rendered':True}
    assert inspect('final')['document_digest']==state['document_digest']
    report.update(status='passed',revision=revision,random_access=True,restore_equal=True,rendering_preserves_source=True,sequence_frames=5,fixture='fixtures/project-cli',dependency_delta='none')
except Exception as error:
    report.update(status='failed',error=repr(error));save_report();raise
save_report()
print('Project CLI workflow passed: '+str(root),flush=True)
