"""Independent animated glTF CLI acceptance client. Fresh output path required."""
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
fixture = Path('fixtures/animated-gltf')
report = {'status': 'in_progress', 'profile': 'gltf2-animated-pbr-v0', 'calls': [],
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
    call('init',{'method':'init','document_id':identifier(99)})
    settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
    settings.update(width=64,height=64,samples=8,max_depth=1)
    settings['camera'].update(position=[1,1,5],target=[0,1,0])
    request={'method':'import','base_revision':inspect()['revision'],'idempotency_key':'animated:import:01','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(fixture/'character.glb'),'policy':{'allow_approximations':True}},'settings':settings}
    imported=call('import',request,resource=True)
    assert call('retry',request)==imported
    state=inspect();revision=state['revision'];clip=imported['import']['source_clips']['0']
    report['import']=imported['import'];report['revision']=revision;report['clip']=clip
    report['serialized_document_bytes']=len(json.dumps(state['snapshot']).encode())
    call('static-defaults',{'method':'render','revision':revision,'backend':'cpu','output':str(root/'default')},resource=True);verify_manifest(root/'default')
    for n in [2,0,4,1,3,2]:
        call('evaluate-'+str(n),{'method':'evaluate','revision':revision,'at':{'clip':clip,'time':rational(n,2)}})
    request={'revision':revision,'clip':clip,'times':[rational(n,2) for n in range(5)],'shutter':shutter}
    call('sequence',{'method':'sequence','request':request,'output':str(root/'sequence')},resource=True)
    assert len(verify_manifest(root/'sequence')['bundles'])==5
    call('frame',{'method':'frame','request':{'revision':revision,'clip':clip,'time':rational(1),'shutter':shutter},'output':str(root/'frame')},resource=True)
    assert (root/'frame/frame/image.pfm').read_bytes()==(root/'sequence/frame-000002/image.pfm').read_bytes()
    call('archive',{'method':'export','revision':revision,'output':str(root/'archive'),'content':{'format':'document'}})
    call('restore',{'method':'restore','source':str(root/'archive/document/document.json')},root/'restored')
    assert inspect('restored',root/'restored')['document_digest']==state['document_digest']
    call('restored-frame',{'method':'frame','request':{'revision':revision,'clip':clip,'time':rational(1),'shutter':shutter},'output':str(root/'restored-frame')},root/'restored')
    assert (root/'restored-frame/frame/image.pfm').read_bytes()==(root/'frame/frame/image.pfm').read_bytes()
    sample={'clip':clip,'time':rational(1)}
    call('cpu',{'method':'render','revision':revision,'at':sample,'backend':'cpu','output':str(root/'cpu')},resource=True)
    if '--gpu' in sys.argv:
        call('gpu',{'method':'render','revision':revision,'at':sample,'backend':'gpu','output':str(root/'gpu')},resource=True)
        a=pfm(root/'cpu/image/image.pfm')[1];b=pfm(root/'gpu/image/image.pfm')[1]
        rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5
        ca=json.loads((root/'cpu/image/passes.json').read_text());ga=json.loads((root/'gpu/image/passes.json').read_text())
        mismatch=sum(x!=y for x,y in zip(ca[4],ga[4],strict=True))
        assert rmse<.025 and mismatch<=4,(rmse,mismatch)
        report['gpu_comparison']={'linear_rmse':rmse,'object_mismatches':mismatch}
    call('stale',{'method':'evaluate','revision':'stale','at':sample},error='stale_revision')
    cancel=root/'cancel';cancel.write_text('cancel')
    call('cancel',{'method':'evaluate','revision':revision,'at':sample},limits={'cancel_file':str(cancel)},error='cancelled')
    call('limited-output',{'method':'render','revision':revision,'at':sample,'backend':'cpu','output':str(root/'limited')},limits={'max_output_bytes':32},error='budget')
    assert inspect('unchanged')['document_digest']==state['document_digest']
    # Render the independently authored external Khronos source through explicit resources.
    external=root/'external';call('external-init',{'method':'init','document_id':identifier(100)},external)
    base=inspect('external-base',external)['revision'];directory=fixture/'khronos-simple-skin'
    gltf=json.loads((directory/'SimpleSkin.gltf').read_text())
    result=call('external-import',{'method':'import','base_revision':base,'idempotency_key':'external:import:01','max_added_bytes':8*1024*1024,'source':{'format':'gltf','path':str(directory/'SimpleSkin.gltf'),'buffers':[str(directory/b['uri']) for b in gltf['buffers']],'images':[],'policy':{'allow_approximations':True}},'settings':settings},external,resource=True)
    external_state=inspect('external-state',external);external_clip=result['import']['source_clips']['0']
    call('external-frame',{'method':'frame','request':{'revision':external_state['revision'],'clip':external_clip,'time':rational(1),'shutter':shutter},'output':str(root/'external-frame')},external,resource=True)
    verify_manifest(root/'external-frame')
    report['external_import']=result['import'];report['status']='passed';save_report()
except BaseException as error:
    report['status']='failed';report['error']=str(error);save_report();raise
