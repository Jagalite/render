"""Collect sparse sculpt displacement evidence and verify all previous engine outputs."""
import hashlib,json,os,shutil,subprocess,sys,tempfile,tomllib
from pathlib import Path
root=Path(sys.argv[1]);dest=Path(sys.argv[2]);baseline='7bd2e25';previous=Path(os.environ['RENDER_SCULPT_PREVIOUS_RUN'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(dest/name).write_text(json.dumps(value,indent=2)+'\n')
assert json.loads((root/'validation_report.json').read_text())['status']=='passed'
subprocess.run(['python3','scripts/compare-sculpt-probe.py',str(root)],check=True)
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
with tempfile.TemporaryDirectory(prefix='render-sculpt-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/sculpt-displacements/generate.py',str(out)],check=True)
 assert {p.name for p in out.iterdir()}=={p.name for p in Path('fixtures/sculpt-displacements/data').iterdir()}
 for p in out.iterdir():
  q=Path('fixtures/sculpt-displacements/data')/p.name;assert p.read_bytes()==q.read_bytes();reproduced.append(dict(path=str(q),sha256=sha(q),reproduced_identically=True))
write('fixture_reproduction.json',reproduced)
shutil.copytree(root/'sculpt-workflow',dest/'sculpt-workflow')
for name in ['static-pbr-probe','sculpt-probe-baseline']:shutil.copytree(root/name,dest/name)
workflow=json.loads((root/'sculpt-workflow/workflow_report.json').read_text());browser=json.loads((root/'sculpt-workflow/browser_report.json').read_text())
assert sha(Path(workflow['tested_native']['path']))==workflow['tested_native']['sha256']
assert sha(Path('web/pkg/render_web_bg.wasm'))==browser['package_sha256']
write('representation_costs.json',dict(scope='Local copy/refit counts are separate from global document admission, hashing, instance bounds and GPU host packing. Actual layouts exclude allocator and Arc headers and private map allocation layout; basis/current can share chunks. Process peaks are whole one-shot CLI observations, not comparative benchmarks.',portable=json.loads((root/'sculpt_resources.json').read_text()),browser=browser['costs'],processes=[row for row in workflow['calls'] if 'maximum_resident_bytes' in row],initial_document_bytes=(root/'sculpt-workflow/initial-archive/document/document.json').stat().st_size,final_document_bytes=(root/'sculpt-workflow/archive/document/document.json').stat().st_size))
# Query results and all qualified chunk counters must stay exact after this change.
for relative in ['chunk_resources.json','spatial-workflow/cross_platform.json']:
 assert json.loads((root/relative).read_text())==json.loads((previous/relative).read_text()),relative
old_queries=json.loads((previous/'spatial-workflow/workflow_report.json').read_text())['queries']
queries=json.loads((root/'spatial-workflow/workflow_report.json').read_text())['queries']
assert old_queries==queries,'prior spatial requests/results/costs changed'
write('prior_spatial_and_chunk_contracts.json',dict(status='passed',queries=len(queries),query_requests_results_costs_exact=True,chunk_resources_exact=True))
development=dest/'development';development.mkdir()
for p in (root.parent/'development').iterdir():
 if p.is_file() and p.suffix in ['.log','.json','.py','.md']:shutil.copy2(p,development/p.name)
for p in (root.parent/'development').glob('workflow*'):
 if p.is_dir():shutil.copytree(p,development/p.name)
for candidate in root.parent.glob('run-*'):
 if candidate==root or not (candidate/'validation_report.json').exists():continue
 if json.loads((candidate/'validation_report.json').read_text()).get('status')!='failed':continue
 folder=development/candidate.name;folder.mkdir()
 for name in ['validation_report.json','source_manifest.json']:shutil.copy2(candidate/name,folder/name)
 shutil.copytree(candidate/'logs',folder/'logs')
 if (candidate/'sculpt-workflow').exists():shutil.copytree(candidate/'sculpt-workflow',folder/'sculpt-workflow')
write('collection.json',dict(status='passed',baseline=baseline,render_artifact_count=len(regressions),prior_fixture_files=len(fixtures),reproduced_fixture_files=len(reproduced),native_calls=len(workflow['calls'])))
print(json.dumps(dict(status='passed',evidence=str(dest),regressions=len(regressions),prior_fixtures=len(fixtures))))
