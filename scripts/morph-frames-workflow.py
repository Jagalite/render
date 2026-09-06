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
fixture = Path('fixtures/morph-frames/data')
report = {'status': 'in_progress', 'profile': 'gltf2-morph-frames-v1', 'calls': [],
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
    settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings']
    settings.update(width=32,height=32,samples=8,max_depth=2)
    settings['camera'].update(position=[0,0,5],target=[0,0,0],lens={'kind':'orthographic','xmag':2,'ymag':2.5,'near':0.1,'far':10})
    report['settings']=settings;report['variants']=[]
    for variant in ['dense','sparse']:
        target=root/variant;call(variant+'-init',{'method':'init','document_id':identifier(10000)},target)
        operation={'method':'import','base_revision':inspect(variant+'-base',target)['revision'],'idempotency_key':'morphframe:import:01','max_added_bytes':8*1024*1024,'source':{'format':'glb','path':str(fixture/(variant+'.glb')),'policy':{'allow_approximations':True}},'settings':settings}
        imported=call(variant+'-import',operation,target,resource=True);assert call(variant+'-retry',operation,target)==imported
        state=inspect(variant+'-state',target);revision=state['revision'];clip=imported['import']['source_clips']['0']
        request={'revision':revision,'clip':clip,'time':rational(1,2),'shutter':shutter}
        for backend in ['cpu','gpu']:
            call(variant+'-'+backend,{'method':'frame','backend':backend,'request':request,'output':str(root/(variant+'-'+backend))},target,resource=True);verify_manifest(root/(variant+'-'+backend))
        a=pfm(root/(variant+'-cpu')/'frame/image.pfm')[1];b=pfm(root/(variant+'-gpu')/'frame/image.pfm')[1]
        rmse=(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**.5
        ca=json.loads((root/(variant+'-cpu')/'frame/passes.json').read_text());ga=json.loads((root/(variant+'-gpu')/'frame/passes.json').read_text());assert ca[4]==ga[4] and rmse<.002
        call(variant+'-archive',{'method':'export','revision':revision,'output':str(root/(variant+'-archive')),'content':{'format':'document'}},target)
        recovered=root/(variant+'-restored');call(variant+'-restore',{'method':'restore','source':str(root/(variant+'-archive')/'document/document.json')},recovered)
        call(variant+'-restored-frame',{'method':'frame','backend':'gpu','request':request,'output':str(root/(variant+'-restored-frame'))},recovered)
        assert (root/(variant+'-gpu')/'frame/image.pfm').read_bytes()==(root/(variant+'-restored-frame')/'frame/image.pfm').read_bytes()
        call(variant+'-stale',dict(operation,idempotency_key='morphframe:stale:0001'),target,error='stale_revision')
        assert inspect(variant+'-unchanged',target)['document_digest']==state['document_digest']
        report['variants'].append({'name':variant,'revision':revision,'clip':clip,'linear_rmse':rmse,'object_mismatches':0,'document_bytes':len(json.dumps(state['snapshot']).encode())})
    for variant in ['sparse']:
        assert (root/'dense-cpu/frame/image.pfm').read_bytes()==(root/(variant+'-cpu')/'frame/image.pfm').read_bytes()
        assert (root/'dense-gpu/frame/image.pfm').read_bytes()==(root/(variant+'-gpu')/'frame/image.pfm').read_bytes()
    # Missing base direction travels through the real explicit-resource adapter.
    project=root/'negative';call('negative-init',{'method':'init','document_id':identifier(10000)},project)
    state=inspect('negative-state',project);broken=json.loads((fixture/'dense.gltf').read_text());broken['meshes'][0]['primitives'][0]['attributes'].pop('NORMAL')
    malformed=root/'malformed.gltf';malformed.write_text(json.dumps(broken))
    operation={'method':'import','base_revision':state['revision'],'idempotency_key':'morphframe:negative:01','max_added_bytes':8*1024*1024,'source':{'format':'gltf','path':str(malformed),'buffers':[str(fixture/'dense.bin')],'images':[],'policy':{'allow_approximations':True}},'settings':settings}
    call('malformed',operation,project,error='gltf_scene')
    operation['source']['path']=str(fixture/'dense.gltf')
    cancel=root/'cancel';cancel.write_text('cancel');call('cancelled',operation,project,limits={'cancel_file':str(cancel)},error='cancelled')
    call('budget',operation,project,limits={'max_input_bytes':64},error='budget')
    assert inspect('negative-unchanged',project)['document_digest']==state['document_digest']
    call('explicit-valid',operation,project)
    report['dense_sparse_pixels_identical']=True;report['status']='passed';save_report()
except BaseException as error:
    report['status']='failed';report['error']=str(error);save_report();raise
