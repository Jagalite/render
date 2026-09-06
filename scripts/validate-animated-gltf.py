"""Fresh evidence for animated glTF. Requires isolated browser/server at 9224/8766."""
import datetime,hashlib,json,os,re,shutil,struct,subprocess,time,sys
from pathlib import Path
root=Path('artifacts/animated-gltf')/datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ');logs=root/'logs';logs.mkdir(parents=True)
checks=[
('format',['cargo','fmt','--all','--check']),
('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
('native_tests',['cargo','test','--workspace','--locked','--offline']),
('wasm_tests',['cargo','test','-p','render-core','--target','wasm32-unknown-unknown','--locked','--offline']),
('linux_check',['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),
('windows_check',['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),
('browser_clippy',['cargo','clippy','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline','--','-D','warnings']),
('native_build',['cargo','build','-p','render-host','--locked','--offline']),
('browser_build',['sh','scripts/build-web.sh']),
('dependency_audit',['python3','scripts/dependency-audit.py']),
('cli_regression',['python3','scripts/project-cli-workflow.py',str(root/'cli-regression'),'--gpu']),
('animated_workflow',['python3','scripts/animated-gltf-workflow.py',str(root/'workflow'),'--gpu']),
('browser_workflow',['node','scripts/animated-gltf-browser.mjs',str(root/'workflow')]),
]
report={'status':'in_progress','root':str(root),'checks':[],'scope':'macOS ARM64 CPU/Metal runtime; Rust Wasm tests in Node; Chromium OPFS/CPU/WebGPU runtime; Linux/Windows compile only'}
def save():(root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
env=dict(os.environ,CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner',CARGO_NET_OFFLINE='true')
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
inputs=[]
for name in sorted(set(paths)):
 p=Path(name)
 if p.is_file() and (name.startswith(('crates/','fixtures/')) or name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','scripts/build-web.sh','scripts/dependency-audit.py','scripts/project-cli-workflow.py']):
  data=p.read_bytes();inputs.append({'path':name,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
(root/'validated_inputs.json').write_text(json.dumps({'note':'Runtime and test inputs captured before validation gates.','files':inputs},indent=2)+'\n')
print(root,flush=True)
reuse=None
if sys.argv[1:]:
    assert len(sys.argv)==3 and sys.argv[1]=='--reuse-checks', 'usage: validate-animated-gltf.py [--reuse-checks prior-run]'
    reuse=Path(sys.argv[2])
    verified=json.loads((reuse/'validated_inputs.json').read_text())
    assert {r['path'] for r in verified['files']}=={r['path'] for r in inputs}, 'runtime input set changed'
    for row in verified['files']:
        data=Path(row['path']).read_bytes();assert len(data)==row['bytes'] and hashlib.sha256(data).hexdigest()==row['sha256'],row['path']
    prior=json.loads((reuse/'validation_report.json').read_text())
    reusable={r['name']:r for r in prior['checks'] if r['exit_code']==0 and r['name'] not in ['animated_workflow','browser_workflow']}
    shutil.copy2(reuse/'validated_inputs.json',root/'validated_inputs.json')
for name,cmd in checks:
 if reuse and name in reusable:
  row=dict(reusable[name]);row['reused_from']=str(reuse);report['checks'].append(row)
  shutil.copy2(reuse/row['log'],root/row['log'])
  if name=='cli_regression':shutil.copytree(reuse/'cli-regression',root/'cli-regression')
  save();print(name+': retained passed check with unchanged inputs',flush=True);continue
 at=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env=env)
 (logs/(name+'.txt')).write_text(r.stdout+r.stderr)
 report['checks'].append({'name':name,'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'});save();print(name+': '+str(r.returncode),flush=True)
 if r.returncode:report['status']='failed';save();print((r.stdout+r.stderr)[-6000:],flush=True);raise SystemExit(r.returncode)
def pfm(p):
 h,d,e,b=p.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0'
 values=struct.unpack('<'+'f'*(w*y*3),b);stride=w*3
 return tuple(value for row in range(y-1,-1,-1) for value in values[row*stride:(row+1)*stride])
comparisons=[]
for browser,native,passes in [('browser-frame-1.json','frame/frame/image.pfm','frame/frame/passes.json'),('browser-default-gpu.json','default/image/image.pfm','default/image/passes.json')]:
 b=json.loads((root/'workflow'/browser).read_text());a=pfm(root/'workflow'/native);rgb=[x for p in b['linear_rgb'] for x in p]
 rmse=(sum((x-y)**2 for x,y in zip(a,rgb,strict=True))/len(a))**.5
 ids=json.loads((root/'workflow'/passes).read_text())[4];mismatches=sum(x!=y for x,y in zip(ids,b['object_ids'],strict=True))
 assert rmse<.025 and mismatches<=4,(rmse,mismatches)
 comparisons.append({'browser':browser,'native':native,'linear_rmse':rmse,'object_mismatches':mismatches})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
for kind in ['native','wasm']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
manifest=[{'path':n,'bytes':Path(n).stat().st_size,'sha256':hashlib.sha256(Path(n).read_bytes()).hexdigest()} for n in sorted(set(paths)) if Path(n).is_file() and not n.startswith(('artifacts/','evidence/'))]
(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
report['status']='passed';save();print('Passed: '+str(root),flush=True)
