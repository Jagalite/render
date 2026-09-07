"""Collect spatial-query evidence and verify compatibility with GPU buffer updates."""
import hashlib,json,os,shutil,subprocess,sys,tempfile,tomllib
from pathlib import Path
root=Path(sys.argv[1]);dest=Path(sys.argv[2]);baseline='4e8441c';previous=Path(os.environ['RENDER_SPATIAL_PREVIOUS_RUN'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(dest/name).write_text(json.dumps(value,indent=2)+'\n')
assert json.loads((root/'validation_report.json').read_text())['status']=='passed'
assert not dest.exists();dest.mkdir(parents=True)
for p in root.glob('*.json'):shutil.copy2(p,dest/p.name)
shutil.copytree(root/'logs',dest/'logs')
regressions=[]
for p in previous.rglob('*'):
 if p.is_file() and p.name in ['image.pfm','cpu.pfm','image.ppm','image.png','passes.json','receipt.json']:
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
cargo=[]
for name in subprocess.check_output(['git','ls-tree','-r','--name-only',baseline],text=True).splitlines():
 if name=='Cargo.lock' or name=='Cargo.toml' or name.endswith('/Cargo.toml'):
  p=Path(name);before=subprocess.check_output(['git','show',baseline+':'+name]);assert before==p.read_bytes(),name
  cargo.append(dict(path=name,sha256=sha(p),unchanged=True))
write('dependency_delta.json',dict(baseline=baseline,new_packages=[],resolved_packages_features_and_native_links_unchanged=True,computational_ffi_added=False,cargo_files=cargo))
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
for row in json.loads(Path('evidence/gpu-geometry-updates/shader_provenance.json').read_text())['artifacts']:
 data=subprocess.check_output(row['generator']);assert hashlib.sha256(data).hexdigest()==row['sha256'];(dest/row['artifact']).write_bytes(data);shaders.append(row)
write('shader_provenance.json',dict(baseline=baseline,artifacts=shaders,all_unchanged=True))
reproduced=[]
with tempfile.TemporaryDirectory(prefix='render-spatial-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/spatial-queries/generate.py',str(out)],check=True)
 assert {p.name for p in out.iterdir()}=={p.name for p in Path('fixtures/spatial-queries/data').iterdir()}
 for p in out.iterdir():
  q=Path('fixtures/spatial-queries/data')/p.name;assert p.read_bytes()==q.read_bytes();reproduced.append(dict(path=str(q),sha256=sha(q),reproduced_identically=True))
write('fixture_reproduction.json',reproduced)
shutil.copytree(root/'spatial-workflow',dest/'spatial-workflow')
workflow=json.loads((root/'spatial-workflow/workflow_report.json').read_text())
browser=json.loads((root/'spatial-workflow/browser_report.json').read_text())
assert sha(Path(workflow['tested_native']['path']))==workflow['tested_native']['sha256']
assert sha(Path('web/pkg/render_web_bg.wasm'))==browser['package_sha256']
costs=[]
for native,web in zip(workflow['queries'],browser['queries'],strict=True):
 n=native['result']['cost'];w=web['cost']
 assert n['retained_index_layout_bytes']<=n['build_byte_charge']
 assert w['retained_index_layout_bytes']<=w['build_byte_charge']
 costs.append(dict(query=native['request']['query'],native=n,browser=w))
write('representation_costs.json',dict(scope='Portable build charges and query traversal are separate from global document validation/hash costs. Actual retained index layout excludes source, Arc allocation headers and allocator overhead. CLI process peaks include project loading and global validation.',observations=costs,processes=[row for row in workflow['calls'] if 'maximum_resident_bytes' in row],initial_document_bytes=(root/'spatial-workflow/initial-archive/document/document.json').stat().st_size,final_document_bytes=(root/'spatial-workflow/archive/document/document.json').stat().st_size))
development=dest/'development';development.mkdir()
for p in (root.parent/'development').iterdir():
 if p.is_file() and p.suffix in ['.log','.json','.py','.rs','.md']:shutil.copy2(p,development/p.name)
for p in list((root.parent/'development').glob('workflow*')) + [root.parent/'development/tiny-probe',root.parent/'development/conditioned-probe']:
 if p.is_dir():shutil.copytree(p,development/p.name)
for candidate in root.parent.glob('run-*'):
 if candidate==root or not (candidate/'validation_report.json').exists():continue
 if json.loads((candidate/'validation_report.json').read_text()).get('status')!='failed':continue
 folder=development/candidate.name;folder.mkdir()
 for name in ['validation_report.json','source_manifest.json']:shutil.copy2(candidate/name,folder/name)
 shutil.copytree(candidate/'logs',folder/'logs')
 if (candidate/'spatial-workflow').exists():shutil.copytree(candidate/'spatial-workflow',folder/'spatial-workflow')
write('collection.json',dict(status='passed',baseline=baseline,render_artifact_count=len(regressions),prior_fixture_files=len(fixtures),new_fixture_files=len(reproduced),native_queries=len(workflow['queries'])))
print(json.dumps(dict(status='passed',evidence=str(dest),regressions=len(regressions),prior_fixtures=len(fixtures))))
