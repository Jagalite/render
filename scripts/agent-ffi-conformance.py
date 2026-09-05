"""Independent Python consumer: real asset -> three scoped variants -> selection."""
import ctypes as c
import hashlib
import json
from pathlib import Path
import subprocess
import time

started = time.monotonic()
root = Path('artifacts/m05')
root.mkdir(parents=True, exist_ok=True)
lib = c.CDLL(str(Path('target/debug/librender_ffi.dylib').resolve()))
Create = c.CFUNCTYPE(c.c_uint64, c.c_uint64, c.c_uint64)
Destroy = c.CFUNCTYPE(c.c_int32, c.c_uint64)
Request = c.CFUNCTYPE(c.c_uint64, c.c_uint64, c.c_void_p, c.c_uint64)
Size = c.CFUNCTYPE(c.c_uint64, c.c_uint64)
Read = c.CFUNCTYPE(c.c_int32, c.c_uint64, c.c_void_p, c.c_uint64)
class Table(c.Structure):
    _fields_ = [('size', c.c_uint32), ('version', c.c_uint32), ('create', Create), ('destroy', Destroy), ('request', Request), ('response_size', Size), ('read', Read), ('release', Destroy)]
lib.render_entry.argtypes = [c.c_uint32, c.c_uint32]
lib.render_entry.restype = c.POINTER(Table)
api = lib.render_entry(1, c.sizeof(Table)).contents
assert api.version == 1 and api.size == c.sizeof(Table)
assert not lib.render_entry(2, 0)
assert not lib.render_entry(1, c.sizeof(Table)+1)
engine = api.create(0, 600)
peak_response = 0
def request(payload, error=None):
    global peak_response
    data = json.dumps(payload).encode()
    buffer = api.request(engine, data, len(data))
    assert buffer
    length = api.response_size(buffer)
    peak_response = max(peak_response, length)
    out = c.create_string_buffer(length)
    assert api.read(buffer, out, length-1) == -2
    assert api.read(buffer, out, length) == 0
    assert api.release(buffer) == 0
    assert api.release(buffer) == -1
    result = json.loads(out.raw)
    if error:
        assert result['Err']['code'] == error, result
        return result['Err']
    assert 'Ok' in result, result
    return result['Ok']
def agent(method, error=None, **args):
    return request({'method':'agent','request':{'version':0,'operation':{'method':method,**args}}}, error)
initial = agent('inspect')
settings = {'width':32,'height':32,'samples':4,'seed':42,'max_depth':1,'max_bytes':8388608,'environment':[0.15]*3,'light':{'position':[2,3,4],'intensity':[20]*3},'camera':{'position':[2,1.5,3],'target':[0,0,0],'up':[0,1,0],'vertical_fov_radians':0.6}}
asset = Path('fixtures/khronos-box/Box.glb').read_bytes()
import_args = dict(bytes=list(asset),policy={'allow_lambertian':True},settings=settings,base_revision=initial['revision'],idempotency_key='ffi:import:000001')
imported = agent('import_glb', **import_args)
assert imported == agent('import_glb', **import_args)
base = agent('inspect')
material = next(iter(base['snapshot']['materials']))
rows = []
for name, color in [('warm',[0.8,0.1,0.1]),('green',[0.1,0.8,0.1]),('cool',[0.1,0.1,0.8])]:
    b = agent('branch',branch=name,base_revision=base['revision'])
    assert b == agent('branch',branch=name,base_revision=base['revision'])
    edits = {'materials':[{'material':material,'base_color':color,'emission':[0,0,0]}], 'light':settings['light'],'environment':[0.2]*3}
    args = dict(branch=name,base_revision=b['revision'],idempotency_key=f'ffi:modify:{name}:0001',edits=edits,max_added_bytes=1048576)
    modified = agent('modify',**args)
    assert modified == agent('modify',**args)
    bad = dict(args,idempotency_key=f'ffi:forbidden:{name}',edits=dict(edits,geometry={}))
    agent('modify',error='encoding',**bad)
    preview = agent('preview',branch=name,revision=modified['revision'])
    assert preview['protected_digest'] == base['protected_digest']
    assert preview['inspection']['visible_pixels'] > 100
    assert len(preview['passes']['object_ids']) == 1024
    rows.append({'name':name,'revision':modified['revision'],'receipt':preview['receipt'],'inspection':preview['inspection']})
agent('branch',error='budget',branch='fourth',base_revision=base['revision'])
assert len({row['receipt']['output_digest'] for row in rows}) == 3
selected = max(rows, key=lambda r:r['inspection']['mean_linear_rgb'][1])
assert selected['name'] == 'green'
prepared = agent('prepare_commit',branch=selected['name'],revision=selected['revision'],idempotency_key='ffi:selection:0001',max_added_bytes=1048576)
commit_args = dict(branch=selected['name'],revision=selected['revision'],idempotency_key='ffi:selection:0001',max_added_bytes=1048576)
receipt = agent('commit',**commit_args)
assert receipt == agent('commit',**commit_args)
# Same prepared transaction can be retried through the general root API.
assert request({'method':'execute','request':prepared}) == receipt
final = agent('inspect')
assert final['revision'] == selected['revision'] and final['protected_digest'] == base['protected_digest']
assert agent('export')['document']['snapshot']['render_settings']['environment'] == [c.c_float(0.2).value]*3
assert api.destroy(engine) == 0 and api.destroy(engine) == -1
subprocess.run(['cc','-std=c11','-Wall','-Wextra','-Werror','-Iinclude','scripts/abi-v1-client.c','-Ltarget/debug','-lrender_ffi','-Wl,-rpath,@executable_path/../../target/debug','-o',str(root/'abi-c-client')],check=True)
c_report = json.loads(subprocess.check_output([str(root/'abi-c-client')],text=True))
report = {'status':'passed','abi_version':1,'scope':'trusted local synchronous ABI; domain operations version 0; explicit buffer ownership; no callbacks', 'clients':['Python ctypes','C11'],'c_client':c_report,'python_workflow':{'variants':rows,'selected':selected['name'],'receipt':receipt,'import':imported['report']},'source_sha256':hashlib.sha256(asset).hexdigest(),'peak_response_bytes':peak_response,'seconds':time.monotonic()-started}
(root/'abi_v1_conformance.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':'passed','clients':report['clients'],'report':str(root/'abi_v1_conformance.json')}))
