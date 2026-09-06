"""Collect evaluated GLB export observations without changing reference fixtures or images."""
import hashlib,json,re,shutil,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/gltf-export')
report=json.loads((root/'validation_report.json').read_text());assert report['status']=='passed'
def write(name,value): (root/name).write_text(json.dumps(value,indent=2)+'\n')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
previous=Path('/private/tmp/render-vertex-colors-20260906/artifacts/vertex-colors/run-20260906T075516Z')
comparisons=[];receipts=[]
for old,new in [('animated-regression','animated-regression'),('shutter-regression','shutter-regression'),('sparse-regression','sparse-regression'),('alpha-regression','alpha-regression'),('named-uv-regression','named-uv-regression'),('morph-regression','morph-regression'),('workflow','vertex-color-regression')]:
    for p in (previous/old).rglob('*'):
        if p.is_file() and p.name in ['image.ppm','image.pfm','passes.json','receipt.json']:
            current=root/new/p.relative_to(previous/old)
            equal=p.read_bytes()==current.read_bytes()
            assert equal,current
            if p.name=='receipt.json':
                a=json.loads(p.read_text());b=json.loads(current.read_text())
                receipts.append({'path':str(current.relative_to(root)),'byte_identical':equal,'changed_top_level_fields':[k for k in a.keys()|b.keys() if a.get(k)!=b.get(k)]})
            else:
                assert equal,current
                comparisons.append({'path':str(current.relative_to(root)),'sha256':sha(current),'byte_identical':True})
assert comparisons
write('regression_identity.json',{'baseline':'6c3a969','baseline_artifacts':str(previous),'image_and_pass_comparisons':comparisons,'receipts':receipts,'receipt_policy':'All previous images, passes and receipts must remain byte-identical.'})
workflow=json.loads((root/'workflow/workflow_report.json').read_text())
reproduction=[]
for row in workflow['variants']:
    original=Path(row['source']);exported=root/'workflow'/(row['name']+'-export/scene/scene.glb');repeated=root/'workflow'/(row['name']+'-repeat/scene/scene.glb')
    assert exported.read_bytes()==repeated.read_bytes()
    reproduction.append({'name':row['name'],'source':str(original),'source_sha256':sha(original),'export_sha256':sha(exported),'repeat_bytes_identical':True})
write('fixture_integrity.json',{'recipe':'fixtures/gltf-export/cases.json','recipe_sha256':sha(Path('fixtures/gltf-export/cases.json')),'cases':reproduction})
tracked=subprocess.check_output(['git','ls-tree','-r','--name-only','6c3a969'],text=True).splitlines()
fixture_integrity=[];cargo=[]
for name in tracked:
    if name.startswith('fixtures/') or Path(name).name in ['Cargo.toml','Cargo.lock']:
        p=Path(name);assert p.read_bytes()==subprocess.check_output(['git','show','6c3a969:'+name])
        (fixture_integrity if name.startswith('fixtures/') else cargo).append({'path':name,'sha256':sha(p),'unchanged':True})
write('original_fixture_integrity.json',fixture_integrity)
write('dependency_delta.json',{'runtime_dependency_changes':False,'cargo_files':cargo})
manifest=json.loads((root/'source_manifest.json').read_text());rust=[]
for entry in manifest:
    p=Path(entry['path'])
    if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
        assert sha(p)==entry['sha256'],p
        rust.append(dict(entry,unchanged_during_validation=True))
write('source_integrity.json',rust)
packages=[Path('target/debug/render-host'),*Path('web/pkg').glob('*')]
write('tested_package.json',[{'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(packages) if p.is_file()])
workflow=json.loads((root/'workflow/workflow_report.json').read_text())
write('resources.json',{'scope':'32x16, 8 samples, depth2; UV, color, exact-time morph, MASK and BLEND export/reimport cases; individual process observations are not a benchmark or allocator limit','observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in workflow['calls'] if 'resources' in r]})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for name in ['logs','workflow','camera-regression','source-validation']:shutil.copytree(root/name,evidence/name)
for name in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','named-uv-regression','morph-regression','vertex-color-regression']:
    (evidence/name).mkdir();shutil.copy2(root/name/'workflow_report.json',evidence/name/'workflow_report.json')
for candidate in sorted(root.parent.glob('run-*')):
    if candidate==root:continue
    result=candidate/'validation_report.json'
    if not result.exists():continue
    target=evidence/'development-runs'/candidate.name;target.mkdir(parents=True)
    for p in candidate.glob('*.json'):shutil.copy2(p,target/p.name)
    shutil.copytree(candidate/'logs',target/'logs')
    debug=candidate/'workflow/browser-export-debug'
    if debug.exists():shutil.copytree(debug,target/'browser-export-debug')
ordering=root.parent/'ordering-before.txt'
if ordering.exists():shutil.copy2(ordering,evidence/'ordering-before.txt')
for p in evidence.rglob('*.txt'):
    s=p.read_text();p.write_text('\n'.join(line.rstrip() for line in s.splitlines()).rstrip()+'\n' if s.strip() else '')
print(json.dumps({'regression_images_passes_identical':len(comparisons),'original_fixtures_unchanged':len(fixture_integrity),'changed_receipts':sum(not r['byte_identical'] for r in receipts),'evidence':str(evidence)}))
