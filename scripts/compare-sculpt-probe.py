"""Check the adapted native sampler probe against the qualified pre-change binary."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
root=Path(sys.argv[1]);baseline=Path('artifacts/sculpt-displacements/baseline')
provenance=json.loads((baseline/'provenance.json').read_text());host=baseline/'render-host'
assert hashlib.sha256(host.read_bytes()).hexdigest()==provenance['sha256']
old=root/'sculpt-probe-baseline';new=root/'static-pbr-probe'
assert new.exists() and not old.exists()
log=root/'logs/sculpt_probe_baseline.txt'
with log.open('w') as stream:
 result=subprocess.run(['/usr/bin/time','-l',str(host),'pbr-workflow',str(old)],stdout=stream,stderr=subprocess.STDOUT)
assert result.returncode==0,log
comparisons=[]
for p in sorted(old.iterdir()):
 if p.is_file() and (p.suffix in ['.pfm','.ppm'] or p.name.endswith(('.passes.json','.receipt.json')) or p.name in ['sampler_camera_report.json','back_face_report.json']):
  q=new/p.name;assert p.read_bytes()==q.read_bytes(),p.name
  comparisons.append({'path':p.name,'sha256':hashlib.sha256(q.read_bytes()).hexdigest(),'byte_identical':True})
assert len(comparisons)==26,len(comparisons)
observations=[]
for label,path,usage in [('baseline',old,log),('candidate',new,root/'logs/static_pbr_probe.txt')]:
 data=json.loads((path/'workflow_report.json').read_text());assert data['status']=='passed'
 peaks={name:int(re.search(r'(\d+)\s+'+name,usage.read_text()).group(1)) for name in ['maximum resident set size','peak memory footprint']}
 observations.append({'package':label,'process':peaks,'workflow_seconds':data['seconds'],'assets':[{'asset':a['asset'],'cpu_seconds':a['cpu_seconds'],'gpu_seconds':a['gpu_seconds'],'triangles':a['triangles'],'encoded_snapshot_bytes':a['encoded_snapshot_bytes'],'gpu_packed_bytes':a['gpu_packed_bytes']} for a in data['assets']]})
report={'status':'passed','baseline':provenance,'files':comparisons,'resource_observations':observations,'scope':'Sequential whole PBR workflows including GPU initialization and global evaluation. Timing is uncontrolled observational evidence, not a speed comparison or benchmark claim.'}
(root/'sculpt_probe_comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':'passed','byte_identical_files':len(comparisons)}))
