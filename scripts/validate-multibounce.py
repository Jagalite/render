"""Fresh multi-bounce evidence. Local browser/server at 9224/8766 required."""
import datetime,hashlib,json,os,re,shutil,struct,subprocess,time
from pathlib import Path
root=Path('artifacts/multibounce-pbr')/datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ');logs=root/'logs';logs.mkdir(parents=True)
report={'status':'in_progress','scope':'macOS ARM64 CPU/Metal; Rust Wasm in Node; Chromium CPU/WebGPU/OPFS; Linux and Windows compile only','checks':[]}
def save():(root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
env=dict(os.environ,CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner',CARGO_NET_OFFLINE='true')
checks=[('format',['cargo','fmt','--all','--check']),('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),('native_tests',['cargo','test','--workspace','--locked','--offline']),('gpu_tests',['cargo','test','-p','render-host','--test','multibounce_gpu','--locked','--offline','--','--ignored']),('wasm_tests',['cargo','test','-p','render-core','--target','wasm32-unknown-unknown','--locked','--offline']),('linux_check',['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),('windows_check',['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),('browser_clippy',['cargo','clippy','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline','--','-D','warnings']),('native_build',['cargo','build','-p','render-host','--locked','--offline']),('browser_build',['sh','scripts/build-web.sh']),('dependency_audit',['python3','scripts/dependency-audit.py']),('cli_regression',['python3','scripts/project-cli-workflow.py',str(root/'cli-regression'),'--gpu']),('static_regression',['target/debug/render-host','pbr-workflow',str(root/'static-regression')]),('animated_regression',['python3','scripts/animated-gltf-workflow.py',str(root/'animated-regression'),'--gpu']),('multibounce_workflow',['python3','scripts/multibounce-workflow.py',str(root/'workflow')]),('browser_workflow',['node','scripts/multibounce-browser.mjs',str(root/'workflow')])]
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
manifest=[{'path':n,'bytes':Path(n).stat().st_size,'sha256':hashlib.sha256(Path(n).read_bytes()).hexdigest()} for n in sorted(set(paths)) if Path(n).is_file() and not n.startswith(('artifacts/','evidence/'))]
(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(root,flush=True)
for name,cmd in checks:
 at=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env=env);(logs/(name+'.txt')).write_text(r.stdout+r.stderr)
 report['checks'].append({'name':name,'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'});save();print(name+': '+str(r.returncode),flush=True)
 if r.returncode:report['status']='failed';save();print((r.stdout+r.stderr)[-6000:]);raise SystemExit(r.returncode)
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
comparisons=[]
for depth in [1,2,4,16]:
 native=pfm(root/f'workflow/cpu-{depth}/image/image.pfm');passes=json.loads((root/f'workflow/cpu-{depth}/image/passes.json').read_text())
 for backend in ['cpu','gpu']:
  browser=json.loads((root/f'workflow/browser-{backend}-{depth}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p];rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
  assert rmse<.002 and passes[4]==browser['object_ids'],(depth,backend,rmse)
  comparisons.append({'depth':depth,'browser':backend,'linear_rmse':rmse,'object_mismatches':0})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
for kind in ['native','wasm','gpu']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
report['status']='passed';save();print('Passed: '+str(root),flush=True)
