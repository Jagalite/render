"""Validate the bounded static PBR slice; never changes historical evidence/goldens.

Requires the isolated Rust server :8765 and Chrome CDP :9223 (docs/static_pbr.md).
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

root=Path('artifacts/static-pbr');logs=root/'logs';logs.mkdir(parents=True,exist_ok=True)
native=root/datetime.datetime.now(datetime.timezone.utc).strftime('native-%Y%m%dT%H%M%SZ')
checks=[('regressions',['python3','scripts/validate-agent.py']),('native_pbr',['/usr/bin/time','-l','target/debug/render-host','pbr-workflow',str(native)]),('browser_pbr',['node','scripts/static-pbr-browser-conformance.mjs']),('cross_platform',['python3','scripts/static-pbr-compare.py',str(native)])]
report={'status':'in_progress','started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'native_directory':str(native),'checks':[],'scope':'macOS ARM64 CPU/Metal; Chromium WASM/WebGPU; Linux/Windows compile-only'}
for name,command in checks:
    start=time.monotonic();result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (logs/(name+'.txt')).write_text(result.stdout)
    report['checks'].append({'name':name,'command':command,'exit_code':result.returncode,'seconds':time.monotonic()-start,'log':'logs/'+name+'.txt'})
    if name=='native_pbr' and result.returncode==0:
        report['native_resource_usage']={label:int(re.search(r'(\d+)\s+'+label,result.stdout).group(1)) for label in ['maximum resident set size','peak memory footprint']}
    report['status']='failed' if result.returncode else 'in_progress'
    (root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(name+': exit '+str(result.returncode),flush=True)
    if result.returncode:print(result.stdout[-7000:]);raise SystemExit(result.returncode)
# Revalidate original fixture provenance bytes, including attribution resources.
provenance=[]
for path in sorted(Path('fixtures/static-pbr').glob('*/provenance.json')):
    manifest=json.loads(path.read_text())
    for item in manifest['files']:
        data=(path.parent/item['file']).read_bytes()
        if len(data)!=item['bytes'] or hashlib.sha256(data).hexdigest()!=item['sha256']:raise RuntimeError('fixture drift: '+item['file'])
    provenance.append({'manifest':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
report['provenance']=provenance
# Pin maintained inputs and generated outputs; historical M05 evidence remains unchanged.
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
source=[]
for name in sorted(set(paths)):
    path=Path(name)
    if path.is_file() and not name.startswith(('evidence/','artifacts/')):
        source.append({'path':name,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
(root/'source_manifest.json').write_text(json.dumps(source,indent=2)+'\n')
report['status']='passed';report['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
(root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
print('Static PBR validation passed',flush=True)
