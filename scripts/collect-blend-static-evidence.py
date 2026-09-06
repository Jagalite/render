"""Collect static Blender profile evidence without redistributing the upstream source."""
import hashlib,json,shutil,subprocess,sys,tempfile,tomllib
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/blend-static');baseline='9f7b4bd'
previous=Path(sys.argv[2])
report=json.loads((root/'validation_report.json').read_text());assert report['status']=='passed'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(root/name).write_text(json.dumps(value,indent=2)+'\n')
def original(name):return subprocess.check_output(['git','show',baseline+':'+name])
folders=['animated-regression','shutter-regression','sparse-regression','alpha-regression','alpha-shutter','secondary-workflow','surface-workflow','conductor-shutter','coated-shutter','named-uv-regression','morph-regression','vertex-color-regression','workflow','dielectric-workflow','dielectric-shutter','volume-workflow','hair-workflow','curve-color-workflow','geometry-regression','groom-regression','media-workflow','media-shutter']
images=[];receipts=[]
for folder in folders:
 assert (previous/folder).is_dir(),folder
 for p in (previous/folder).rglob('*'):
  if p.is_file() and p.name in ['image.ppm','image.pfm','passes.json','receipt.json']:
   current=root/folder/p.relative_to(previous/folder);assert p.read_bytes()==current.read_bytes(),current
   (receipts if p.name=='receipt.json' else images).append({'path':str(current.relative_to(root)),'sha256':sha(current),'byte_identical':True})
assert images and receipts
write('regression_identity.json',{'baseline':baseline,'baseline_artifacts':str(previous),'image_and_pass_comparisons':images,'receipts':receipts})
tracked=subprocess.check_output(['git','ls-tree','-r','--name-only',baseline],text=True).splitlines();fixtures=[];cargo=[]
for name in tracked:
 if name.startswith('fixtures/'):
  p=Path(name);assert p.read_bytes()==original(name),name;fixtures.append({'path':name,'sha256':sha(p),'unchanged':True})
 elif Path(name).name in ['Cargo.toml','Cargo.lock']:
  p=Path(name);old=tomllib.loads(original(name).decode());new=tomllib.loads(p.read_text())
  if name=='Cargo.lock':
   package=next(x for x in new['package'] if x['name']=='render-core');assert package['dependencies'].count('libm')==1;package['dependencies'].remove('libm')
  elif name=='crates/core/Cargo.toml':assert new['dependencies'].pop('libm')=={'version':'=0.2.16','default-features':False}
  assert old==new,name;cargo.append({'path':name,'sha256':sha(p),'change':'direct libm core edge' if name in ['Cargo.lock','crates/core/Cargo.toml'] else 'unchanged'})
write('original_fixture_integrity.json',fixtures)
audit=json.loads((root/'dependency_inventory.json').read_text());old_audit=json.loads(Path('evidence/gpu-media/dependency_inventory.json').read_text())
for target in ['native','browser']:assert audit[target]['packages']==old_audit[target]['packages'],('resolved dependency delta',target)
write('dependency_delta.json',{'baseline':baseline,'cargo_files':cargo,'new_packages':False,'resolved_features_unchanged':True,'native_links_unchanged':audit['native_link_inspection']==old_audit['native_link_inspection'],'new_direct_dependency':'libm 0.2.16 default-features=false for Blender lens atan only','foreign_computational_runtime':False,'provenance':'planning/engine-progress/blend-libm-provenance/review.json'})
rust=[]
for row in json.loads((root/'source_manifest.json').read_text()):
 p=Path(row['path'])
 if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
  assert sha(p)==row['sha256'],p;rust.append(dict(row,unchanged_during_validation=True))
write('source_integrity.json',rust)
for row in json.loads((root/'tested_package.json').read_text()):assert sha(Path(row['path']))==row['sha256'],row['path']
shaders=[]
for row in json.loads(Path('evidence/gpu-media/shader_provenance.json').read_text())['artifacts']:
 p=root/row['artifact'];p.write_bytes(subprocess.check_output(row['generator']));assert sha(p)==row['sha256'],p
 shaders.append({'artifact':p.name,'sha256':sha(p),'baseline':baseline,'unchanged':True,'generator':row['generator']})
write('shader_provenance.json',{'artifacts':shaders,'inputs':[r for r in rust if r['path'].startswith('crates/kernel/src/')]})
generated=[]
for kind in ['blend-container','blend-static']:
 with tempfile.TemporaryDirectory(prefix='render-blend-reproduce-') as temp:
  out=Path(temp)/'data';subprocess.run(['python3',f'fixtures/{kind}/generate.py',str(out)],check=True)
  assert {p.name for p in out.iterdir()}=={p.name for p in Path(f'fixtures/{kind}/data').iterdir()}
  for p in sorted(out.iterdir()):
   actual=Path(f'fixtures/{kind}/data')/p.name;assert p.read_bytes()==actual.read_bytes(),actual
   generated.append({'path':str(actual),'sha256':sha(actual),'reproduced_identically':True})
write('fixture_reproduction.json',generated)
workflow=json.loads((root/'blend-workflow/workflow_report.json').read_text())
write('resources.json',{'scope':'Per-process import and 32x32 synthetic or 32x18 real CPU/Metal render observations; not benchmarks, allocator caps or broad file-size support','observations':[{'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in workflow['calls'] if 'resources' in r],'source_storage':[{'name':v['name'],'source_bytes':v['report']['source_bytes'],'canonical_asset_bytes':v['report']['source_asset_json_bytes'],'archive_json_bytes':Path(v['archive']).stat().st_size,'meshes':1,'materials':1,'entities':len(v['native_entities'])} for v in workflow['variants']]})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for p in root.glob('*.wgsl'):shutil.copy2(p,evidence/p.name)
shutil.copytree(root/'logs',evidence/'logs')
for folder in folders+['blend-workflow','source-validation']:
 dest=evidence/folder;dest.mkdir(exist_ok=True)
 for name in ['workflow_report.json','browser_report.json','schema_validation.json','validation.json','cross_platform.json','reference_comparison.json']:
  p=root/folder/name
  if p.exists():shutil.copy2(p,dest/name)
# Retain rendered products for every input; exact-source artifacts only for original CC0 cases.
for v in workflow['variants']:
 name=v['name']
 for suffix in ['cpu','gpu','recovered','recovered-gpu']:
  shutil.copytree(root/'blend-workflow'/(name+'-'+suffix),evidence/'blend-workflow'/(name+'-'+suffix))
 for backend in ['cpu','gpu']:
  p=root/'blend-workflow'/('browser-'+name+'-'+backend+'.json');shutil.copy2(p,evidence/'blend-workflow'/p.name)
 if not v['external_reference']:
  for suffix in ['archive','source','recovered-source']:
   shutil.copytree(root/'blend-workflow'/(name+'-'+suffix),evidence/'blend-workflow'/(name+'-'+suffix))
(evidence/'development').mkdir()
research=Path('/private/tmp/render-blend-research-20260906')
for name in ['browser-state-differences.json','browser-startup-differences.json']:
 p=research/name
 if p.exists():shutil.copy2(p,evidence/'development'/name)
p=root.parent/'preflight-source.json'
if p.exists():shutil.copy2(p,evidence/'development'/p.name)
for p in root.parent.glob('*.log'):shutil.copy2(p,evidence/'development'/p.name)
for candidate in root.parent.glob('*dev*'):
 if candidate.is_dir() and (candidate/'workflow_report.json').exists():
  dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'workflow_report.json',dest/'workflow_report.json')
for candidate in root.parent.glob('run-*'):
 if candidate==root:continue
 dest=evidence/'development'/candidate.name;dest.mkdir();shutil.copy2(candidate/'validation_report.json',dest/'validation_report.json')
 if (candidate/'logs').exists():shutil.copytree(candidate/'logs',dest/'logs')
print(json.dumps({'evidence':str(evidence),'previous_images_and_passes':len(images),'previous_receipts':len(receipts),'unchanged_fixtures':len(fixtures),'native_tests':report['native_test_count'],'wasm_tests':report['wasm_test_count'],'blend_calls':len(workflow['calls']),'blend_cases':len(workflow['variants']),'reproduced_files':len(generated)}))
