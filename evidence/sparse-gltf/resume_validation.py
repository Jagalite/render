from pathlib import Path
import json,hashlib,subprocess,time,os,re,shutil,struct
root=Path('artifacts/sparse-gltf/run-20260906T045410Z');report=json.loads((root/'validation_report.json').read_text());assert report['status']=='failed'
initial=json.loads((root/'source_manifest.json').read_text());verified=[]
for item in initial:
 p=item['path']
 if p.startswith(('crates/','fixtures/')) or p in ['Cargo.lock','Cargo.toml','rust-toolchain.toml']:
  assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==item['sha256'],p;verified.append(p)
report.setdefault('resume_history',[]).append(report.get('resume_verification'))
report['resume_verification']={'runtime_tests_and_fixtures_unchanged':True,'verified_files':len(verified),'reason':'Client stale-check key had only 15 characters, below the 16-character minimum. Corrected key and valid pre-import revision; Rust sources and tests unchanged.','client_sha256':hashlib.sha256(Path('scripts/sparse-gltf-workflow.py').read_bytes()).hexdigest()}
report['status']='in_progress';report.setdefault('followup_checks',[])
def save():(root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
workflow=root/'workflow-retry2';env=dict(os.environ,CARGO_NET_OFFLINE='true',CARGO_BUILD_JOBS='2')
for name,cmd in [('sparse_workflow_retry2',['python3','scripts/sparse-gltf-workflow.py',str(workflow)]),('browser_workflow',['node','scripts/sparse-gltf-browser.mjs',str(workflow)])]:
 at=time.monotonic();path=root/'logs'/(name+'.txt')
 with path.open('w') as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 report['followup_checks'].append({'name':name,'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'});save();print(name,r.returncode,flush=True)
 if r.returncode:report['status']='failed';save();print(path.read_text()[-5000:]);raise SystemExit(r.returncode)
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
comparisons=[]
for variant in ['dense','sparse-zero-u16','sparse-base-u8']:
 native=pfm(workflow/f'{variant}-cpu/frame/image.pfm');passes=json.loads((workflow/f'{variant}-cpu/frame/passes.json').read_text())
 for backend in ['cpu','gpu']:
  browser=json.loads((workflow/f'browser-{variant}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p];rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5;assert rmse<.002 and passes[4]==browser['object_ids'],(variant,backend,rmse);comparisons.append({'variant':variant,'browser':backend,'linear_rmse':rmse,'object_mismatches':0})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
for kind in ['native','wasm']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(root/'logs'/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
report['status']='passed';report['resolved_failures']={'sparse_workflow':'sparse_workflow_retry2'};report['completed_workflow']='workflow-retry2';save();print('Passed',root,flush=True)
