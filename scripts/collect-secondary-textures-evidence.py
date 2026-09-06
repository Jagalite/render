"""Collect Secondary texture observations without changing reference fixtures or images."""
import hashlib,json,re,shutil,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/secondary-textures')
report=json.loads((root/'validation_report.json').read_text());assert report['status']=='passed'
def write(name,value): (root/name).write_text(json.dumps(value,indent=2)+'\n')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
previous=Path('/private/tmp/render-gpu-alpha-20260906/artifacts/gpu-alpha/run-20260906T093429Z')
comparisons=[];receipts=[]
for old,new in [('animated-regression','animated-regression'),('shutter-regression','shutter-regression'),('sparse-regression','sparse-regression'),('alpha-regression','alpha-regression'),('alpha-shutter','alpha-shutter'),('named-uv-regression','named-uv-regression'),('morph-regression','morph-regression'),('vertex-color-regression','vertex-color-regression'),('workflow','workflow')]:
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
write('regression_identity.json',{'baseline':'1460e14','baseline_artifacts':str(previous),'image_and_pass_comparisons':comparisons,'receipts':receipts,'receipt_policy':'All previous images, passes and receipts must remain byte-identical.'})
workflow=json.loads((root/'workflow/workflow_report.json').read_text())
reproduction=[]
for row in workflow['variants']:
    original=Path(row['source']);exported=root/'workflow'/(row['name']+'-export/scene/scene.glb');repeated=root/'workflow'/(row['name']+'-repeat/scene/scene.glb')
    assert exported.read_bytes()==repeated.read_bytes()
    reproduction.append({'name':row['name'],'source':str(original),'source_sha256':sha(original),'export_sha256':sha(exported),'repeat_bytes_identical':True})
write('fixture_integrity.json',{'recipe':'fixtures/gltf-export/cases.json','recipe_sha256':sha(Path('fixtures/gltf-export/cases.json')),'cases':reproduction})
tracked=subprocess.check_output(['git','ls-tree','-r','--name-only','1460e14'],text=True).splitlines()
fixture_integrity=[];cargo=[]
for name in tracked:
    if name.startswith('fixtures/') or Path(name).name in ['Cargo.toml','Cargo.lock']:
        p=Path(name)
        assert p.read_bytes()==subprocess.check_output(['git','show','1460e14:'+name])
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
workflow=json.loads((root/'secondary-workflow/workflow_report.json').read_text())
write('resources.json',{'scope':'8x8, 32 samples, depth2; five native-authored surface models, nearest versus mip filtering, CPU plus two supported GPU models; individual process observations are not a benchmark or allocator limit','observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in workflow['calls'] if 'resources' in r]})
shutter=json.loads((root/'alpha-shutter/workflow_report.json').read_text())
write('alpha_shutter_resources.json',{'scope':'2x2, one spatial sample; 65 shared native quad instances, exact-time step translation; resource observations are not benchmarks','observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in shutter['calls'] if 'resources' in r]})
with tempfile.TemporaryDirectory(prefix='render-secondary-reproduce-') as temp:
    out=Path(temp)/'generated'
    subprocess.run(['python3','fixtures/secondary-textures/generate.py',str(out)],check=True)
    generated=[]
    for p in sorted(out.iterdir()):
        actual=Path('fixtures/secondary-textures/data')/p.name;assert p.read_bytes()==actual.read_bytes(),actual
        generated.append({'path':str(actual),'sha256':sha(actual),'reproduced_identically':True})
write('fixture_reproduction.json',generated)
shader=subprocess.check_output(['target/debug/render-host','kernel']);(root/'path.generated.wgsl').write_bytes(shader)
assert hashlib.sha256(shader).hexdigest()=='e0bae9a561867ddc4622b68e9d52635ce3d380bbf6f4aeb109e63b46c8d89247'
alpha_shader=subprocess.check_output(['target/debug/render-host','kernel','--alpha']);(root/'path.alpha.generated.wgsl').write_bytes(alpha_shader)
assert hashlib.sha256(alpha_shader).hexdigest()=='82177b8262a499bad4e9997fe15d04975b9635e0817ea4caeeb38d713a0648dc'
write('shader_provenance.json',{'artifact':'path.generated.wgsl','sha256':hashlib.sha256(shader).hexdigest(),'opaque_baseline':'1460e14; byte-identical generated shader','alpha_artifact':'path.alpha.generated.wgsl','alpha_sha256':hashlib.sha256(alpha_shader).hexdigest(),'generator':'target/debug/render-host kernel [--alpha]','inputs':[r for r in rust if r['path'].startswith('crates/kernel/src/')],'validation':'Naga typed generation plus actual Metal and Chrome WebGPU pipelines'})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
shutil.copy2(root/'path.generated.wgsl',evidence/'path.generated.wgsl')
shutil.copy2(root/'path.alpha.generated.wgsl',evidence/'path.alpha.generated.wgsl')
for name in ['logs','secondary-workflow','camera-regression','source-validation']:shutil.copytree(root/name,evidence/name)
for name in ['animated-regression','shutter-regression','sparse-regression','named-uv-regression','morph-regression','vertex-color-regression','workflow','alpha-shutter','alpha-regression']:
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
shutil.copytree(root.parent/'cli-before',evidence/'cli-before')
for log in root.parent.glob('*.txt'):shutil.copy2(log,evidence/log.name)
for p in evidence.rglob('*.txt'):
    s=p.read_text();p.write_text('\n'.join(line.rstrip() for line in s.splitlines()).rstrip()+'\n' if s.strip() else '')
print(json.dumps({'regression_images_passes_identical':len(comparisons),'original_fixtures_unchanged':len(fixture_integrity),'changed_receipts':sum(not r['byte_identical'] for r in receipts),'evidence':str(evidence)}))
