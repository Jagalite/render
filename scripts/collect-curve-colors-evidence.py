"""Collect polyline RGBA observations and verify immutable source, shader and prior-output baselines."""
import hashlib,json,shutil,struct,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/curve-colors');baseline='0a3ffd1'
report=json.loads((root/'validation_report.json').read_text());assert report['status']=='passed'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,v):(root/name).write_text(json.dumps(v,indent=2)+'\n')
previous=Path('/private/tmp/render-hair-import-20260906/artifacts/hair-import/run-20260906T121018Z')
comparisons=[];receipts=[]
for folder in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','alpha-shutter','secondary-workflow','surface-workflow','conductor-shutter','coated-shutter','named-uv-regression','morph-regression','vertex-color-regression','workflow','dielectric-workflow','dielectric-shutter','volume-workflow','hair-workflow']:
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
write('original_fixture_integrity.json',fixtures);write('dependency_delta.json',{'runtime_dependency_changes':False,'cargo_files':cargo,'foreign_computational_runtime':False,'format_reference_only':'https://www.cemyuksel.com/research/hairmodels/'})
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
write('shader_provenance.json',{'artifacts':shaders,'inputs':[r for r in rust if r['path'].startswith('crates/kernel/src/')]})
with tempfile.TemporaryDirectory(prefix='render-hair-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/curve-colors/generate.py',str(out)],check=True);generated=[]
 for p in sorted(out.iterdir()):
  actual=Path('fixtures/curve-colors/data')/p.name;assert p.read_bytes()==actual.read_bytes();generated.append({'path':str(actual),'sha256':sha(actual),'reproduced_identically':True})
write('fixture_reproduction.json',generated)
old=[]
for label in ['geometry','groom']:
 baseline_dir=root.parent/('baseline-'+label);current_dir=root/(label+'-regression')
 for p in baseline_dir.rglob('*'):
  if p.is_file() and p.suffix in ['.json','.pfm','.ppm'] and p.name!='report.json':
   current=current_dir/p.relative_to(baseline_dir);assert p.read_bytes()==current.read_bytes(),current
   old.append({'workflow':label,'path':str(p.relative_to(baseline_dir)),'sha256':sha(p),'identical_to_previous_binary':True})
write('uncolored_workflow_identity.json',{'baseline_binary':json.loads((root.parent/'baseline-binary.json').read_text()),'artifacts':old})
workflow=json.loads((root/'curve-color-workflow/workflow_report.json').read_text());write('resources.json',{'scope':'HAIR 16x16 and authored groom32x32,16 samples,depth2; individual process observations are not benchmarks or allocator caps','observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in workflow['calls'] if 'resources' in r]})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for p in root.glob('*.wgsl'):shutil.copy2(p,evidence/p.name)
for folder in ['logs','curve-color-workflow','source-validation']:shutil.copytree(root/folder,evidence/folder)
for folder in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','alpha-shutter','secondary-workflow','surface-workflow','conductor-shutter','coated-shutter','named-uv-regression','morph-regression','vertex-color-regression','workflow','dielectric-workflow','dielectric-shutter','volume-workflow','hair-workflow']:
 (evidence/folder).mkdir()
 for name in ['workflow_report.json','browser_report.json','schema_validation.json','validation.json']:
  p=root/folder/name
  if p.exists():shutil.copy2(p,evidence/folder/name)
(evidence/'development').mkdir()
for p in root.parent.glob('*.log'):shutil.copy2(p,evidence/'development'/p.name)
for candidate in root.parent.glob('workflow-dev*'):
 if candidate.is_dir() and (candidate/'workflow_report.json').exists():
  dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'workflow_report.json',dest/'workflow_report.json')
for candidate in root.parent.glob('run-*'):
 if candidate==root:continue
 dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'validation_report.json',dest/'validation_report.json')
 if (candidate/'logs').exists():shutil.copytree(candidate/'logs',dest/'logs')
print(json.dumps({'evidence':str(evidence),'previous_images_and_passes':len(comparisons),'previous_receipts':len(receipts),'unchanged_fixtures':len(fixtures),'native_tests':report['native_test_count'],'wasm_tests':report['wasm_test_count'],'curve_color_calls':len(workflow['calls']),'reproduced_files':len(generated)}))
