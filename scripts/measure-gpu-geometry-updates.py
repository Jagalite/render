"""Observe the actual conformance test process separately from Cargo compilation."""
import hashlib,json,os,re,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]);proof=json.loads((root/'gpu-update-workflow/tested_native_gpu.json').read_text());binary=Path(proof['path']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==proof['sha256']
env=dict(os.environ,RENDER_GPU_UPDATE_OUTPUT=str((root/'gpu-update-resource-workflow').resolve()))
at=time.monotonic()
with (root/'logs/gpu_resource_process.txt').open('w') as output:
 result=subprocess.run(['/usr/bin/time','-l',str(binary),'--ignored','--nocapture'],env=env,stdout=output,stderr=subprocess.PIPE,text=True)
(root/'logs/gpu_resource_time.txt').write_text(result.stderr)
assert result.returncode==0,result.stderr
rss=re.search(r'^\s*(\d+)\s+maximum resident set size',result.stderr,re.M);assert rss
report=dict(status='passed',seconds=time.monotonic()-at,maximum_resident_set_bytes=int(rss[1]),executable=proof,scope='Observed macOS conformance process, including fixture transactions, evaluation, CPU renders, multiple fresh GPU adapters and artifact serialization; excludes Cargo compilation and is not a performance benchmark or allocator cap.')
(root/'gpu_update_resources.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
