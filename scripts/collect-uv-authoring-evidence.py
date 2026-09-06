"""Collect a passed UV candidate and verify its exact preexisting artifacts."""
import hashlib,json,os,shutil,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);dest=Path(sys.argv[2]);baseline='c9b0188';previous=Path(os.environ['RENDER_UV_PREVIOUS_RUN'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(dest/name).write_text(json.dumps(value,indent=2)+'\n')
assert json.loads((root/'validation_report.json').read_text())['status']=='passed'
assert not dest.exists();dest.mkdir(parents=True)
for p in root.glob('*.json'):shutil.copy2(p,dest/p.name)
shutil.copytree(root/'logs',dest/'logs')
regressions=[]
for p in previous.rglob('*'):
 if p.is_file() and (p.name in ['image.pfm','image.ppm','image.png','passes.json'] or p.name=='receipt.json'):
  rel=p.relative_to(previous);new=root/rel
  assert new.exists() and p.read_bytes()==new.read_bytes(),str(rel)
  regressions.append({'path':str(rel),'sha256':sha(p),'unchanged':True})
assert regressions
write('render_regressions.json',{'baseline':baseline,'files':regressions,'byte_identical':True})
fixtures=[]
for name in subprocess.check_output(['git','ls-tree','-r','--name-only',baseline,'fixtures'],text=True).splitlines():
 p=Path(name);old=subprocess.check_output(['git','show',baseline+':'+name]);assert p.read_bytes()==old,name
 fixtures.append({'path':name,'sha256':sha(p),'unchanged':True})
write('original_fixture_integrity.json',fixtures)
current=json.loads((root/'dependency_inventory.json').read_text());old=json.loads((previous/'dependency_inventory.json').read_text())
for target in ['native','browser']:assert current[target]['packages']==old[target]['packages'],target
assert current['native_link_inspection']==old['native_link_inspection']
for name in subprocess.check_output(['git','ls-tree','-r','--name-only',baseline],text=True).splitlines():
 if name=='Cargo.lock' or name=='Cargo.toml' or name.endswith('/Cargo.toml'):assert Path(name).read_bytes()==subprocess.check_output(['git','show',baseline+':'+name]),name
write('dependency_delta.json',{'baseline':baseline,'cargo_files_unchanged':True,'resolved_packages_features_and_native_links_unchanged':True,'new_dependencies':[],'computational_ffi_added':False})
for row in json.loads((root/'tested_package.json').read_text()):assert sha(Path(row['path']))==row['sha256']
compiled=[]
for row in json.loads((root/'source_manifest.json').read_text()):
 p=Path(row['path'])
 if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
  assert sha(p)==row['sha256'],p;compiled.append(row)
write('source_integrity.json',compiled)
shaders=[]
for row in json.loads(Path('evidence/blend-static/shader_provenance.json').read_text())['artifacts']:
 data=subprocess.check_output(row['generator']);assert hashlib.sha256(data).hexdigest()==row['sha256'],row
 (dest/row['artifact']).write_bytes(data);shaders.append(row)
write('shader_provenance.json',{'baseline':baseline,'artifacts':shaders,'all_unchanged':True})
reproduced=[]
with tempfile.TemporaryDirectory(prefix='render-uv-reproduce-') as temp:
 out=Path(temp)/'data';subprocess.run(['python3','fixtures/uv-authoring/generate.py',str(out)],check=True)
 assert {p.name for p in out.iterdir()}=={p.name for p in Path('fixtures/uv-authoring/data').iterdir()}
 for p in out.iterdir():
  actual=Path('fixtures/uv-authoring/data')/p.name;assert p.read_bytes()==actual.read_bytes();reproduced.append({'path':str(actual),'sha256':sha(actual),'reproduced_identically':True})
write('fixture_reproduction.json',reproduced)
workflow=json.loads((root/'uv-workflow/workflow_report.json').read_text());resources=[{'name':row['name'],'seconds':row['seconds'],**row['resources']} for row in workflow['calls'] if 'resources' in row]
write('resources.json',{'scope':'Observed per-process native authoring and 32x32 rendering on this host; not performance benchmarks or process memory caps','observations':resources,'operations':[row['result']['report'] for row in workflow['operations']]})
uv=dest/'uv-workflow';uv.mkdir()
for p in (root/'uv-workflow').glob('*'):
 if p.is_file():shutil.copy2(p,uv/p.name)
for name in ['archive','source-cpu','box-cpu','box-gpu','box-restored']:shutil.copytree(root/'uv-workflow'/name,uv/name)
development=dest/'development';development.mkdir()
for p in root.parent.glob('*.log'):shutil.copy2(p,development/p.name)
for name in ['input-review-corrections.json']:
 p=root.parent/name
 if p.exists():shutil.copy2(p,development/name)
for candidate in root.parent.glob('run-*'):
 if candidate==root or not (candidate/'validation_report.json').exists():continue
 failed=json.loads((candidate/'validation_report.json').read_text())
 if failed.get('status')!='failed':continue
 folder=development/candidate.name;folder.mkdir()
 for name in ['validation_report.json','source_manifest.json']:
  if (candidate/name).exists():shutil.copy2(candidate/name,folder/name)
 shutil.copytree(candidate/'logs',folder/'logs')
write('collection.json',{'status':'passed','baseline':baseline,'render_artifact_count':len(regressions),'prior_fixture_files':len(fixtures),'new_fixture_files':len(reproduced),'native_calls':len(workflow['calls'])})
print(json.dumps({'status':'passed','evidence':str(dest),'regressions':len(regressions)}))
