"""Collect observed alpha acceptance, preserving raw logs and prior fixture bytes."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root=Path(sys.argv[1]); evidence=Path('evidence/alpha-gltf')
report=json.loads((root/'validation_report.json').read_text())
assert report['status']=='passed'
def write(name,value): (root/name).write_text(json.dumps(value,indent=2)+'\n')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
previous=Path('artifacts/sparse-gltf/run-20260906T045410Z')
comparisons=[]
for old,new in [('animated-regression','animated-regression'),('shutter-regression','shutter-regression'),('workflow-retry2','sparse-regression')]:
    for p in (previous/old).rglob('*'):
        if p.is_file() and p.name in ['image.ppm','image.pfm','passes.json','receipt.json']:
            current=root/new/p.relative_to(previous/old)
            assert p.read_bytes()==current.read_bytes(),current
            comparisons.append({'path':str(current.relative_to(root)),'sha256':sha(current),'byte_identical':True})
write('regression_identity.json',{'baseline':'2fc718f','baseline_artifacts':str(previous),'comparisons':comparisons})
with tempfile.TemporaryDirectory(prefix='render-alpha-fixture-') as d:
    output=Path(d)/'data';subprocess.run(['python3','fixtures/alpha-gltf/generate.py',str(output)],check=True)
    fixtures=[]
    for p in sorted(Path('fixtures/alpha-gltf/data').iterdir()):
        assert p.read_bytes()==(output/p.name).read_bytes()
        fixtures.append({'path':str(p),'sha256':sha(p),'reproduced_exactly':True})
    write('fixture_integrity.json',fixtures)
tracked=subprocess.check_output(['git','ls-files'],text=True).splitlines()
fixture_integrity=[]
for name in tracked:
    if name.startswith('fixtures/'):
        p=Path(name);assert p.read_bytes()==subprocess.check_output(['git','show','2fc718f:'+name])
        fixture_integrity.append({'path':name,'sha256':sha(p),'unchanged':True})
write('original_fixture_integrity.json',fixture_integrity)
cargo=[]
for name in tracked:
    if Path(name).name in ['Cargo.toml','Cargo.lock']:
        p=Path(name);assert p.read_bytes()==subprocess.check_output(['git','show','2fc718f:'+name])
        cargo.append({'path':name,'sha256':sha(p),'unchanged':True})
write('dependency_delta.json',{'runtime_dependency_changes':False,'cargo_files':cargo})
manifest=json.loads((root/'source_manifest.json').read_text())
rust=[]
for entry in manifest:
    p=Path(entry['path'])
    if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
        assert sha(p)==entry['sha256'],p
        rust.append(dict(entry,unchanged_during_validation=True))
write('source_integrity.json',rust)
packages=[Path('target/debug/render-host'),*Path('web/pkg').glob('*')]
write('tested_package.json',[{'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(packages) if p.is_file()])
workflow=json.loads((root/'workflow/workflow_report.json').read_text())
write('resources.json',{'scope':'32x16, 64 samples, depth1, two original triangles pairs and 4x1 RGBA; process RSS is not a benchmark or allocator limit',
    'observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in workflow['calls'] if 'resources' in r]})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for name in ['logs','workflow','camera-regression']:shutil.copytree(root/name,evidence/name)
interrupted=Path('artifacts/alpha-gltf/run-20260906T052852Z')
(evidence/'initial-candidate').mkdir()
for p in interrupted.glob('*.json'):shutil.copy2(p,evidence/'initial-candidate'/p.name)
shutil.copytree(interrupted/'logs',evidence/'initial-candidate/logs')
baseline=Path('artifacts/alpha-baseline/run-20260906T050222Z')
shutil.copytree(baseline,evidence/'camera-baseline')
for name in ['animated-regression','shutter-regression','sparse-regression']:
    (evidence/name).mkdir();shutil.copy2(root/name/'workflow_report.json',evidence/name/'workflow_report.json')
# Raw logs stay in artifacts; trim only redundant EOF blank lines in committed copies.
for p in evidence.rglob('*.txt'):
    s=p.read_text();p.write_text(s.rstrip()+'\n' if s.strip() else '')
print(json.dumps({'regression_artifacts_identical':len(comparisons),'original_fixtures_unchanged':len(fixture_integrity),'evidence':str(evidence)}))
