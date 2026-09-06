"""Validate the native project CLI and publish a fresh local evidence run.

No reference evidence is overwritten. Native GPU/resource access is required.
Shared Rust/browser sources are checked against the already validated base commit.
"""
import datetime
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import time

root=Path('artifacts/project-cli')/datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ')
logs=root/'logs';logs.mkdir(parents=True)
checks=[
 ('format',['cargo','fmt','--all','--check']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
 ('native_tests',['cargo','test','--workspace','--locked','--offline']),
 ('linux_check',['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),
 ('windows_check',['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),
 ('browser_check',['cargo','check','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline']),
 ('native_build',['cargo','build','-p','render-host','--locked','--offline']),
 ('dependency_audit',['python3','scripts/dependency-audit.py']),
 ('workflow',['python3','scripts/project-cli-workflow.py',str(root/'workflow'),'--gpu'])]
report={'status':'in_progress','root':str(root),'checks':[], 'base':'d2b34a6','scope':'macOS ARM64 native CPU/Metal runtime; Linux/Windows host compile; browser compile plus unchanged shared-core proof against prior native/Wasm validation.'}
def save(): (root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
print(root,flush=True)
for name,command in checks:
 at=time.monotonic();result=subprocess.run(command,capture_output=True,text=True)
 (logs/(name+'.txt')).write_text(result.stdout+result.stderr)
 report['checks'].append({'name':name,'command':command,'exit_code':result.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'})
 if result.returncode:report['status']='failed'
 save();print(name+': exit '+str(result.returncode),flush=True)
 if result.returncode:
  print((result.stdout+result.stderr)[-7000:],flush=True);raise SystemExit(result.returncode)
# Review only this bounded interface: shared computational/browser code and deps remain unchanged.
base_paths=subprocess.check_output(['git','ls-tree','-r','--name-only','d2b34a6','crates/core','crates/gpu','crates/kernel','crates/web','crates/ffi','Cargo.lock','Cargo.toml'],text=True).splitlines()
unchanged=[]
for name in base_paths:
 original=subprocess.check_output(['git','show','d2b34a6:'+name])
 current=Path(name).read_bytes();assert current==original,name
 unchanged.append({'path':name,'sha256':hashlib.sha256(current).hexdigest()})
(root/'shared_source_integrity.json').write_text(json.dumps({'status':'passed','base':'d2b34a6','prior_wasm_tests':77,'prior_evidence':'evidence/m06-m08/acceptance_index.json','files':unchanged},indent=2)+'\n')
provenance=json.loads(Path('fixtures/project-cli/provenance.json').read_text())
for row in provenance['files']:
 b=(Path('fixtures/project-cli')/row['file']).read_bytes();assert len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256']
report['native_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/'native_tests.txt').read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
manifest=[]
for name in sorted(set(paths)):
 p=Path(name)
 if p.is_file() and not name.startswith(('artifacts/','evidence/')):manifest.append({'path':name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
report['status']='passed';report['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();save()
print('Project CLI validation passed: '+str(root),flush=True)
