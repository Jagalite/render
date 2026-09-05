"""Independent newline-JSON client; exercise actual process disconnect/reconnect."""
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(tempfile.mkdtemp(prefix='native-client-', dir='artifacts/m05'))
exe = 'target/debug/render-host'
def start(mode):
    return subprocess.Popen([exe,mode,str(root/'project')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
def send(proc,payload):
    proc.stdin.write(json.dumps(payload)+'\n');proc.stdin.flush()
    result=json.loads(proc.stdout.readline())
    assert 'Ok' in result, result
    return result['Ok']
def stop(proc):
    proc.stdin.close(); assert proc.wait(timeout=30)==0, proc.stderr.read()
proc=start('agent')
def agent(method,**args):
    return send(proc,{'version':0,'operation':{'method':method,**args}})
base=agent('inspect')
settings={'width':16,'height':16,'samples':2,'seed':42,'max_depth':1,'max_bytes':8388608,'environment':[0.2]*3,'light':{'position':[2,3,4],'intensity':[20]*3},'camera':{'position':[2,1.5,3],'target':[0,0,0],'up':[0,1,0],'vertical_fov_radians':0.6}}
imported=agent('import_glb',bytes=list(Path('fixtures/khronos-box/Box.glb').read_bytes()),policy={'allow_lambertian':True},settings=settings,base_revision=base['revision'],idempotency_key='native:import:0001')
assert imported['receipt']['durable']
base=agent('inspect');material=next(iter(base['snapshot']['materials']))
agent('branch',branch='selected',base_revision=base['revision'])
modified=agent('modify',branch='selected',base_revision=base['revision'],idempotency_key='native:modify:0001',edits={'materials':[{'material':material,'base_color':[0.1,0.8,0.2],'emission':[0,0,0]}],'light':settings['light'],'environment':[0.3]*3},max_added_bytes=1048576)
agent('preview',branch='selected',revision=modified['revision'])
args=dict(branch='selected',revision=modified['revision'],idempotency_key='native:selection:01',max_added_bytes=1048576)
prepared=agent('prepare_commit',**args)
# Send a complete selection and disconnect without consuming its acknowledgment.
proc.stdin.write(json.dumps({'version':0,'operation':{'method':'commit',**args}})+'\n');proc.stdin.flush()
stop(proc)
retry=start('api');receipt=send(retry,prepared);stop(retry)
assert receipt['durable'] and receipt['revision']==modified['revision']
# An interrupted control message must never publish a partial transaction.
broken=start('api');broken.stdin.write(json.dumps(prepared)[:50]);broken.stdin.flush();broken.kill();broken.wait(timeout=10)
oversized=start('agent');_, diagnostic=oversized.communicate('x'*(16*1024*1024+1),timeout=30);assert oversized.returncode==1 and 'budget' in diagnostic
proc=start('agent');recovered=agent('inspect');stop(proc)
assert recovered['revision']==receipt['revision'] and recovered['protected_digest']==base['protected_digest']
assert recovered['branches']==[]
report={'status':'passed','client':'Python newline JSON subprocess','lost_commit_ack_retry':True,'partial_input_disconnect':True,'bounded_stream_admission':True,'selected_document_recovered':True,'receipt':receipt,'prepared_request':prepared}
Path('artifacts/m05/native_client_report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':'passed','report':'artifacts/m05/native_client_report.json'}))
