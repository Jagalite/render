"""Independent trusted ABI client; test tooling, not bundled application runtime."""
import ctypes as c
import json
from pathlib import Path

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
assert not lib.render_entry(1, c.sizeof(Table))
api = lib.render_entry(0, c.sizeof(Table)).contents
assert api.size == c.sizeof(Table)
engine = api.create(1, 2)
def request(payload):
    data = json.dumps(payload).encode()
    response = api.request(engine, data, len(data))
    assert response
    length = api.response_size(response)
    output = c.create_string_buffer(length)
    assert api.read(response, output, length-1) == -2
    assert api.read(response, output, length) == 0
    result = json.loads(output.raw)
    assert api.release(response) == 0
    assert api.response_size(response) == 0
    return result
inspection = request({'method':'inspect'})['Ok']
payload = {'version':0,'base_revision':inspection['revision'],'idempotency_key':'ffi:create:000001','max_added_bytes':10000,'commands':[{'operation':'create_entity','entity':{'id':'0'*31+'1','name':'From external client','parent':None,'mesh':None,'material':None,'transform':{'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}}}]}
receipt = request({'method':'execute','request':payload})['Ok']
assert receipt == request({'method':'execute','request':payload})['Ok']
assert receipt['durable'] is False
assert request({'method':'inspect'})['Ok']['snapshot']['entities'][0]['name'] == 'From external client'
assert len(request({'method':'registry'})['Ok']) == 10
assert api.destroy(engine) == 0
assert api.destroy(engine) == -1
report = {'status':'passed','abi_version':0,'independent_client':'Python ctypes','checks':['version negotiation','structure size','buffer ownership','capacity bounds','released buffer','create/inspect/transaction','idempotent retry','stale engine','registry'],'durability':'memory-only alpha'}
Path('artifacts/evidence').mkdir(parents=True,exist_ok=True)
Path('artifacts/evidence/abi_conformance.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
