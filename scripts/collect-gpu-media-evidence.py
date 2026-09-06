"""Collect GPU sparse media observations and verify immutable source, shader and prior-output baselines."""
import hashlib,json,shutil,struct,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/gpu-media');baseline='35ea08b'
report=json.loads((root/'validation_report.json').read_text());assert report['status']=='passed'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,v):(root/name).write_text(json.dumps(v,indent=2)+'\n')
previous=Path('/private/tmp/render-curve-colors-20260906/artifacts/curve-colors/run-20260906T130429Z')
comparisons=[];receipts=[]
for folder in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','alpha-shutter','secondary-workflow','surface-workflow','conductor-shutter','coated-shutter','named-uv-regression','morph-regression','vertex-color-regression','workflow','dielectric-workflow','dielectric-shutter','volume-workflow','hair-workflow','curve-color-workflow','geometry-regression','groom-regression']:
 for p in (previous/folder).rglob('*'):
  if p.is_file() and p.name in ['image.ppm','image.pfm','passes.json','receipt.json']:
   current=root/folder/p.relative_to(previous/folder);assert p.read_bytes()==current.read_bytes(),current
   (receipts if p.name=='receipt.json' else comparisons).append({'path':str(current.relative_to(root)),'sha256':sha(current),'byte_identical':True})
assert comparisons and receipts
write('regression_identity.json',{'baseline':baseline,'baseline_artifacts':str(previous),'image_and_pass_comparisons':comparisons,'receipts':receipts})
tracked=subprocess.check_output(['git','ls-tree','-r','--name-only',baseline],text=True).splitlines();fixtures=[];cargo=[]
for name in tracked:
 if name.startswith('fixtures/') or Path(name).name in ['Cargo.toml','Cargo.lock']:
  p=Path(name);assert p.read_bytes()==subprocess.check_output(['git','show',baseline+':'+name]);(fixtures if name.startswith('fixtures/') else cargo).append({'path':name,'sha256':sha(p),'unchanged':True})
write('original_fixture_integrity.json',fixtures);write('dependency_delta.json',{'runtime_dependency_changes':False,'cargo_files':cargo,'foreign_computational_runtime':False,'fixture_origin':'original CC0 analytic corpus; existing VOL3 fixtures retained'})
rust=[]
for row in json.loads((root/'source_manifest.json').read_text()):
 p=Path(row['path'])
 if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
  assert sha(p)==row['sha256'],p;rust.append(dict(row,unchanged_during_validation=True))
write('source_integrity.json',rust)
packages=[Path('target/debug/render-host'),*Path('web/pkg').glob('*')];write('tested_package.json',[{'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(packages) if p.is_file()])
shaders=[]
for args,name,expected in [([], 'path.generated.wgsl','e0bae9a561867ddc4622b68e9d52635ce3d380bbf6f4aeb109e63b46c8d89247'),(['--alpha'],'path.alpha.generated.wgsl','82177b8262a499bad4e9997fe15d04975b9635e0817ea4caeeb38d713a0648dc'),(['--surfaces'],'path.surfaces.generated.wgsl','cf5b558cef24671d760df416ccd1b82b7b8166bc26dbf2dfd4e2d3d616808f07'),(['--dielectric'],'path.dielectric.generated.wgsl','afa6f881610497e4c8cdf63f6a28c10ff2a243a23c4f23dcac7aae5179130eea')]:
 data=subprocess.check_output(['target/debug/render-host','kernel',*args]);p=root/name;p.write_bytes(data);assert sha(p)==expected;shaders.append({'artifact':name,'sha256':expected,'baseline':baseline,'unchanged':True,'generator':['target/debug/render-host','kernel',*args]})
data=subprocess.check_output(['target/debug/render-host','kernel','--media']);p=root/'path.media.generated.wgsl';p.write_bytes(data);shaders.append({'artifact':p.name,'sha256':sha(p),'new':True,'generator':['target/debug/render-host','kernel','--media']})
write('shader_provenance.json',{'artifacts':shaders,'inputs':[r for r in rust if r['path'].startswith('crates/kernel/src/')]})
with tempfile.TemporaryDirectory(prefix='render-media-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/gpu-media/generate.py',str(out)],check=True);generated=[]
 for p in sorted(out.iterdir()):
  actual=Path('fixtures/gpu-media/data')/p.name;assert p.read_bytes()==actual.read_bytes();generated.append({'path':str(actual),'sha256':sha(actual),'reproduced_identically':True})
write('fixture_reproduction.json',generated)
workflow=json.loads((root/'media-workflow/workflow_report.json').read_text());write('resources.json',{'scope':'Analytic1x1 and surface16x16 media, external VOL3 and shutter renders; individual process observations are not benchmarks or allocator caps','observations':[{'workflow':folder,'name':r['name'],'seconds':r['seconds'],**r['resources']} for folder in ['media-workflow','volume-workflow','media-shutter'] for r in json.loads((root/folder/'workflow_report.json').read_text())['calls'] if 'resources' in r]})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for p in root.glob('*.wgsl'):shutil.copy2(p,evidence/p.name)
for folder in ['logs','media-workflow','media-shutter','volume-workflow','source-validation']:shutil.copytree(root/folder,evidence/folder)
for folder in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','alpha-shutter','secondary-workflow','surface-workflow','conductor-shutter','coated-shutter','named-uv-regression','morph-regression','vertex-color-regression','workflow','dielectric-workflow','dielectric-shutter','volume-workflow','hair-workflow','curve-color-workflow','geometry-regression','groom-regression']:
 (evidence/folder).mkdir(exist_ok=True)
 for name in ['workflow_report.json','browser_report.json','schema_validation.json','validation.json']:
  p=root/folder/name
  if p.exists():shutil.copy2(p,evidence/folder/name)
(evidence/'development').mkdir()
for p in root.parent.glob('*.log'):shutil.copy2(p,evidence/'development'/p.name)
for candidate in root.parent.glob('*dev*'):
 if candidate.is_dir() and (candidate/'workflow_report.json').exists():
  dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'workflow_report.json',dest/'workflow_report.json')
for candidate in root.parent.glob('run-*'):
 if candidate==root:continue
 dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'validation_report.json',dest/'validation_report.json')
 if (candidate/'logs').exists():shutil.copytree(candidate/'logs',dest/'logs')
print(json.dumps({'evidence':str(evidence),'previous_images_and_passes':len(comparisons),'previous_receipts':len(receipts),'unchanged_fixtures':len(fixtures),'native_tests':report['native_test_count'],'wasm_tests':report['wasm_test_count'],'media_calls':len(workflow['calls']),'reproduced_files':len(generated)}))
