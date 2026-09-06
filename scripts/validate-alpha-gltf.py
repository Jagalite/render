"""Fresh alpha glTF acceptance; local Chrome/server required at 9224/8766."""
import datetime, hashlib, json, os, re, shutil, struct, subprocess, time
from pathlib import Path
root=Path('artifacts/alpha-gltf')/datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ')
logs=root/'logs';logs.mkdir(parents=True)
report={'status':'in_progress','scope':'macOS ARM64 CPU/Metal, Rust Wasm in Node, Chrome WebGPU/OPFS; Linux and Windows compile only','checks':[]}
def save(): (root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
env=dict(os.environ,CARGO_NET_OFFLINE='true',CARGO_BUILD_JOBS='2',CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner')
report['build_environment']={k:env[k] for k in ['CARGO_NET_OFFLINE','CARGO_BUILD_JOBS','CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER']}
checks=[('format',['cargo','fmt','--all','--check']),('alpha_regressions',['cargo','test','-p','render-core','--test','alpha_gltf','--locked','--offline']),('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),('native_tests',['cargo','test','--workspace','--locked','--offline']),('wasm_tests',['cargo','test','-p','render-core','--target','wasm32-unknown-unknown','--locked','--offline']),('linux_check',['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),('windows_check',['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),('browser_clippy',['cargo','clippy','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline','--','-D','warnings']),('native_build',['cargo','build','-p','render-host','--locked','--offline']),('browser_build',['sh','scripts/build-web.sh']),('camera_regression',['python3','scripts/alpha-camera-workflow.py',str(root/'camera-regression')]),('dependency_audit',['python3','scripts/dependency-audit.py']),('animated_regression',['python3','scripts/animated-gltf-workflow.py',str(root/'animated-regression'),'--gpu']),('shutter_regression',['python3','scripts/gpu-shutter-workflow.py',str(root/'shutter-regression')]),('sparse_regression',['python3','scripts/sparse-gltf-workflow.py',str(root/'sparse-regression')]),('alpha_workflow',['python3','scripts/alpha-gltf-workflow.py',str(root/'workflow')]),('browser_workflow',['node','scripts/alpha-gltf-browser.mjs',str(root/'workflow')])]
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
manifest=[{'path':n,'bytes':Path(n).stat().st_size,'sha256':hashlib.sha256(Path(n).read_bytes()).hexdigest()} for n in sorted(set(paths)) if Path(n).is_file() and not n.startswith(('artifacts/','evidence/'))]
(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(root,flush=True)
for name,cmd in checks:
    at=time.monotonic();path=logs/(name+'.txt')
    with path.open('w') as log: r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
    report['checks'].append({'name':name,'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'});save();print(name+': '+str(r.returncode),flush=True)
    if r.returncode: report['status']='failed';save();print(path.read_text()[-6000:]);raise SystemExit(r.returncode)
def pfm(path):
    h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0'
    v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
comparisons=[]
for variant in ['mask','blend','opaque']:
    native=pfm(root/f'workflow/{variant}-cpu/image/image.pfm');passes=json.loads((root/f'workflow/{variant}-cpu/image/passes.json').read_text())
    for backend in ['cpu']:
        browser=json.loads((root/f'workflow/browser-{variant}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p]
        rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
        assert rmse<.002 and passes[4]==browser['object_ids'],(variant,backend,rmse)
        comparisons.append({'variant':variant,'browser':backend,'linear_rmse':rmse,'object_mismatches':0})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
for kind in ['native','wasm']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
report['status']='passed';save();print('Passed: '+str(root),flush=True)
