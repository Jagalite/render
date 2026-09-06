"""Collect passed GPU update evidence and verify compatibility with painting."""
import hashlib,json,os,shutil,subprocess,sys,tempfile,tomllib
from pathlib import Path
root=Path(sys.argv[1]);dest=Path(sys.argv[2]);baseline='24de830';previous=Path(os.environ['RENDER_GPU_UPDATES_PREVIOUS_RUN'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(dest/name).write_text(json.dumps(value,indent=2)+'\n')
assert json.loads((root/'validation_report.json').read_text())['status']=='passed'
assert not dest.exists();dest.mkdir(parents=True)
for p in root.glob('*.json'):shutil.copy2(p,dest/p.name)
shutil.copytree(root/'logs',dest/'logs')
regressions=[]
for p in previous.rglob('*'):
 if p.is_file() and p.name in ['image.pfm','image.ppm','image.png','passes.json','receipt.json']:
  rel=p.relative_to(previous);q=root/rel;assert q.exists() and p.read_bytes()==q.read_bytes(),str(rel)
  regressions.append(dict(path=str(rel),sha256=sha(q),unchanged=True))
assert regressions;write('render_regressions.json',dict(baseline=baseline,byte_identical=True,files=regressions))
fixtures=[]
for name in subprocess.check_output(['git','ls-tree','-r','--name-only',baseline,'fixtures'],text=True).splitlines():
 p=Path(name);assert p.read_bytes()==subprocess.check_output(['git','show',baseline+':'+name]),name;fixtures.append(dict(path=name,sha256=sha(p),unchanged=True))
write('original_fixture_integrity.json',fixtures)
old=json.loads((previous/'dependency_inventory.json').read_text());current=json.loads((root/'dependency_inventory.json').read_text())
for target in ['native','browser']:assert old[target]['packages']==current[target]['packages'],target
assert old['native_link_inspection']==current['native_link_inspection']
old_lock=tomllib.loads(subprocess.check_output(['git','show',baseline+':Cargo.lock'],text=True));new_lock=tomllib.loads(Path('Cargo.lock').read_text())
for package in old_lock['package']:
 if package['name']=='render-web':package['dependencies'].append('serde');package['dependencies'].sort()
assert old_lock==new_lock
cargo=[]
for name in subprocess.check_output(['git','ls-tree','-r','--name-only',baseline],text=True).splitlines():
 if name=='Cargo.lock' or name=='Cargo.toml' or name.endswith('/Cargo.toml'):
  p=Path(name);before=subprocess.check_output(['git','show',baseline+':'+name]);after=p.read_bytes()
  if name=='crates/web/Cargo.toml':assert after==before.replace(b'[target.\'cfg(target_arch = "wasm32")\'.dependencies]\n',b'[target.\'cfg(target_arch = "wasm32")\'.dependencies]\nserde.workspace = true\n')
  elif name!='Cargo.lock':assert before==after,name
  cargo.append(dict(path=name,before_sha256=hashlib.sha256(before).hexdigest(),after_sha256=sha(p),unchanged=before==after))
write('dependency_delta.json',dict(baseline=baseline,new_packages=[],resolved_packages_features_and_native_links_unchanged=True,direct_edge_added='render-web Wasm target -> existing workspace serde 1.0.228 derive/rc',computational_ffi_added=False,cargo_files=cargo))
packages=json.loads((root/'tested_package.json').read_text())+[json.loads((root/'gpu-update-workflow/tested_native_gpu.json').read_text())]
for row in packages:assert sha(Path(row['path']))==row['sha256'],row['path']
assert sha(Path('web/pkg/render_web_bg.wasm'))==json.loads((root/'gpu-update-workflow/browser_report.json').read_text())['package_sha256']
write('tested_package.json',packages)
compiled=[]
for row in json.loads((root/'source_manifest.json').read_text()):
 p=Path(row['path'])
 if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:assert sha(p)==row['sha256'],p;compiled.append(row)
write('source_integrity.json',compiled)
shaders=[]
for row in json.loads(Path('evidence/tiled-painting/shader_provenance.json').read_text())['artifacts']:
 data=subprocess.check_output(row['generator']);assert hashlib.sha256(data).hexdigest()==row['sha256'];(dest/row['artifact']).write_bytes(data);shaders.append(row)
write('shader_provenance.json',dict(baseline=baseline,artifacts=shaders,all_unchanged=True))
reproduced=[]
with tempfile.TemporaryDirectory(prefix='render-gpu-update-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/gpu-geometry-updates/generate.py',str(out)],check=True)
 assert {p.name for p in out.iterdir()}=={p.name for p in Path('fixtures/gpu-geometry-updates/data').iterdir()}
 for p in out.iterdir():
  q=Path('fixtures/gpu-geometry-updates/data')/p.name;assert p.read_bytes()==q.read_bytes();reproduced.append(dict(path=str(q),sha256=sha(q),reproduced_identically=True))
write('fixture_reproduction.json',reproduced)
for p in (root/'gpu-update-workflow').rglob('*'):
 if p.is_file() and p.name in ['image.pfm','cpu.pfm','passes.json','receipt.json','document.json']:
  q=root/'gpu-update-resource-workflow'/p.relative_to(root/'gpu-update-workflow');assert p.read_bytes()==q.read_bytes(),str(q)
for name in ['gpu-update-workflow','gpu-update-resource-workflow']:shutil.copytree(root/name,dest/name)
workflow=json.loads((root/'gpu-update-workflow/native_report.json').read_text());costs=[];prior=0
for row in workflow['cases']:
 stats=row['statistics'];packed=stats['last']['packed_buffer_bytes'];costs.append(dict(name=row['name'],canonical_document_bytes=(root/'gpu-update-workflow'/row['name']/'document.json').stat().st_size,packed_host_record_bytes=packed,serialized_host_bytes=packed,prior_retained_shadow_bytes=prior,logical_host_bytes_at_comparison=2*packed+prior,retained_shadow_after_bytes=stats['retained_shadow_bytes'],uploaded_gpu_bytes=stats['last']['uploaded_bytes']));prior=stats['retained_shadow_bytes']
write('representation_costs.json',dict(scope='Logical serialized/packed payload bytes and retained shadow, not allocator sizes or a process peak; GPU staging, driver allocations, canonical/evaluated meshes and Vec metadata are additional.',observations=costs))
development=dest/'development';development.mkdir()
for p in (root.parent/'development').iterdir():
 if p.is_file() and p.suffix in ['.log','.json','.py','.rs','.md']:shutil.copy2(p,development/p.name)
for candidate in root.parent.glob('run-*'):
 if candidate==root or not (candidate/'validation_report.json').exists():continue
 if json.loads((candidate/'validation_report.json').read_text()).get('status')!='failed':continue
 folder=development/candidate.name;folder.mkdir()
 for name in ['validation_report.json','source_manifest.json']:shutil.copy2(candidate/name,folder/name)
 shutil.copytree(candidate/'logs',folder/'logs')
write('collection.json',dict(status='passed',baseline=baseline,render_artifact_count=len(regressions),prior_fixture_files=len(fixtures),new_fixture_files=len(reproduced),native_cases=len(workflow['cases'])))
print(json.dumps(dict(status='passed',evidence=str(dest),regressions=len(regressions),prior_fixtures=len(fixtures))))
