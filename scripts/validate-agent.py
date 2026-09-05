"""M05 validation; writes new evidence, never changes reference images.

Requires the isolated local Rust web server :8765 and Chrome CDP :9223.
Run from the repository root after starting them as documented in docs/agent_alpha.md.
"""
import datetime
import json
import os
from pathlib import Path
import re
import subprocess
import time

root = Path('artifacts/m05')
logs = root / 'logs'
logs.mkdir(parents=True, exist_ok=True)
run_name = datetime.datetime.now(datetime.timezone.utc).strftime('native-%Y%m%dT%H%M%SZ')
native = root / run_name
checks = [
    ('format', ['cargo','fmt','--all','--check']),
    ('native_clippy', ['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
    ('native_tests', ['cargo','test','--workspace','--locked','--offline']),
    ('wasm_clippy', ['cargo','clippy','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline','--','-D','warnings']),
    ('wasm_tests', ['cargo','test','-p','render-core','--target','wasm32-unknown-unknown','--locked','--offline']),
    ('linux_check', ['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),
    ('windows_check', ['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),
    ('native_build', ['cargo','build','-p','render-host','-p','render-ffi','--locked','--offline']),
    ('abi_header', ['python3','scripts/generate-abi-header.py','--check']),
    ('ffi_v0', ['python3','scripts/ffi-conformance.py']),
    ('ffi_v1', ['python3','scripts/agent-ffi-conformance.py']),
    ('native_client', ['python3','scripts/agent-native-client.py']),
    ('web_build', ['sh','scripts/build-web.sh']),
    ('dependency_audit', ['python3','scripts/dependency-audit.py']),
    ('native_workflow', ['/usr/bin/time','-l','target/debug/render-host','agent-workflow','fixtures/khronos-box/Box.glb',str(native),'--gpu']),
    ('browser_workflow', ['node','scripts/agent-browser-conformance.mjs']),
    ('browser_foundation', ['node','scripts/browser-conformance.mjs']),
]
report = {'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'checks':[], 'native_workflow_directory':str(native), 'scope':'macOS ARM64 and Chromium runtime; Linux/Windows compile-only; no performance portability claim'}
env = dict(os.environ, CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner')
for name, command in checks:
    start=time.monotonic()
    result=subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (logs/(name+'.txt')).write_text(result.stdout)
    report['checks'].append({'name':name,'command':command,'exit_code':result.returncode,'seconds':time.monotonic()-start,'log':'logs/'+name+'.txt'})
    report['status']='passed' if len(report['checks'])==len(checks) and all(c['exit_code']==0 for c in report['checks']) else 'in_progress' if result.returncode==0 else 'failed'
    if name=='native_workflow' and result.returncode==0:
        report['native_resource_usage']={label:int(re.search(r'(\d+)\s+'+label,result.stdout).group(1)) for label in ['maximum resident set size','peak memory footprint']}
    (root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'{name}: exit {result.returncode}',flush=True)
    if result.returncode:
        print(result.stdout[-7000:],flush=True)
        raise SystemExit(result.returncode)
