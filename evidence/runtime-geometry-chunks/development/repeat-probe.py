import hashlib,json,re,subprocess
from pathlib import Path
root=Path('artifacts/runtime-geometry-chunks/run-20260907T023350Z');out=root/'repeat-probes';out.mkdir()
base=Path('artifacts/runtime-geometry-chunks/baseline/render-host');new=Path('artifacts/runtime-geometry-chunks/qualified/render-host')
reference=root/'chunk-probe-baseline';report={'status':'in_progress','scope':'Four additional alternating whole-process probes after an initial candidate/baseline pair. GPU/OS allocation and timing are uncontrolled; all reference outputs must remain exact.','samples':[]}
for i,(label,host) in enumerate([('baseline',base),('candidate',new),('candidate',new),('baseline',base)]):
 folder=out/f'{i}-{label}';log=out/f'{i}-{label}.log'
 with log.open('w') as stream:
  r=subprocess.run(['/usr/bin/time','-l',str(host),'pbr-workflow',str(folder)],stdout=stream,stderr=subprocess.STDOUT)
 assert r.returncode==0,log
 comparisons=[]
 for p in reference.iterdir():
  if p.is_file() and (p.suffix in ['.pfm','.ppm'] or p.name.endswith(('.passes.json','.receipt.json')) or p.name in ['sampler_camera_report.json','back_face_report.json']):
   assert p.read_bytes()==(folder/p.name).read_bytes(),p.name;comparisons.append(p.name)
 assert len(comparisons)==26
 data=json.loads((folder/'workflow_report.json').read_text());usage=log.read_text()
 report['samples'].append({'package':label,'sha256':hashlib.sha256(host.read_bytes()).hexdigest(),'resident_bytes':int(re.search(r'(\d+)\s+maximum resident set size',usage).group(1)),'footprint_bytes':int(re.search(r'(\d+)\s+peak memory footprint',usage).group(1)),'seconds':data['seconds'],'reference_files_exact':len(comparisons)})
 (out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report['samples'][-1]),flush=True)
report['status']='passed';(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
