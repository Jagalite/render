"""Collect source-conformance correction and distinguish identity from rendering."""
import hashlib,json,shutil,subprocess,sys,tempfile
from pathlib import Path
root=Path(sys.argv[1]);evidence=Path('evidence/indexed-attributes')
assert json.loads((root/'validation_report.json').read_text())['status']=='passed'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,v):(root/name).write_text(json.dumps(v,indent=2)+'\n')
previous=Path('/private/tmp/render-morph-frames-20260906/artifacts/morph-frames/run-20260906T071435Z')
identical=[];corrected=[]
for directory in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','named-uv-regression','workflow']:
    affected=directory in ['named-uv-regression','workflow']
    for old in (previous/directory).rglob('*'):
        if not old.is_file() or old.name not in ['image.ppm','image.pfm','passes.json','receipt.json']:continue
        new=root/old.relative_to(previous)
        equal=old.read_bytes()==new.read_bytes()
        row={'path':str(new.relative_to(root)),'sha256':sha(new),'byte_identical':equal}
        if not affected or old.name in ['image.ppm','image.pfm']:
            assert equal,new;identical.append(row);continue
        a=json.loads(old.read_text());b=json.loads(new.read_text())
        if old.name=='passes.json':
            assert a[:4]==b[:4] and len(a[4])==len(b[4]),new
            mapping={}
            for x,y in zip(a[4],b[4],strict=True):
                if x is None:assert y is None
                elif x in mapping:assert mapping[x]==y
                else:mapping[x]=y
            assert len(mapping)==1 and len(set(mapping.values()))==1,(new,mapping)
            row.update(color_depth_normals_unchanged=True,source_scoped_id_mapping=mapping)
        else:
            changed=[k for k in a.keys()|b.keys() if a.get(k)!=b.get(k)]
            assert set(changed)<= {'revision','output_digest'},(new,changed)
            row['changed_fields']=changed
        corrected.append(row)
write('render_identity.json',{'baseline':'d107a5f','unchanged_artifacts':identical,'corrected_source_artifacts':corrected,'policy':'Only source-scoped IDs and dependent receipt revision/output digests may change for the corrected named-UV/morph fixtures; original color/depth/normal values remain exact.'})
tracked=subprocess.check_output(['git','ls-tree','-r','--name-only','d107a5f'],text=True).splitlines()
allowed={'fixtures/named-uv/'+p for p in ['README.md','generate.py','data/provenance.json','data/roles.gltf','data/roles.glb']}|{'fixtures/morph-frames/'+p for p in ['README.md','generate.py','data/provenance.json','data/dense.gltf','data/dense.glb','data/sparse.gltf','data/sparse.glb']}
fixtures=[];cargo=[]
for name in tracked:
    if not (name.startswith('fixtures/') or Path(name).name in ['Cargo.lock','Cargo.toml']):continue
    p=Path(name);old=subprocess.check_output(['git','show','d107a5f:'+name]);equal=old==p.read_bytes()
    assert equal or name in allowed,name
    if p.suffix=='.bin':assert equal,name
    row={'path':name,'unchanged':equal,'old_sha256':hashlib.sha256(old).hexdigest(),'sha256':sha(p)}
    (fixtures if name.startswith('fixtures/') else cargo).append(row)
write('fixture_delta.json',{'files':fixtures,'allowed_metadata_and_generator_corrections':sorted(allowed),'all_binary_payloads_unchanged':True})
write('dependency_delta.json',{'runtime_dependency_changes':False,'cargo_files':cargo,'development_tool':'gltf-validator@2.0.0-dev.3.10; Apache-2.0; installed only in temporary directory; official source-validation oracle'})
reproduced=[]
for family in ['named-uv','morph-frames']:
    with tempfile.TemporaryDirectory(prefix='render-indexed-reproduce-') as d:
        output=Path(d)/'data';subprocess.run(['python3',f'fixtures/{family}/generate.py',str(output)],check=True)
        for p in sorted(Path(f'fixtures/{family}/data').iterdir()):
            assert p.read_bytes()==(output/p.name).read_bytes(),p
            reproduced.append({'path':str(p),'sha256':sha(p),'reproduced_exactly':True})
write('fixture_reproduction.json',reproduced)
manifest=json.loads((root/'source_manifest.json').read_text());source=[]
for r in manifest:
    p=Path(r['path'])
    if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:
        assert sha(p)==r['sha256'],p;source.append(dict(r,unchanged_during_validation=True))
write('source_integrity.json',source)
write('tested_package.json',[{'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)} for p in [Path('target/debug/render-host'),*sorted(Path('web/pkg').glob('*'))] if p.is_file()])
observations=[]
for folder in ['named-uv-regression','workflow']:
    w=json.loads((root/folder/'workflow_report.json').read_text())
    observations.extend({'workflow':folder,'name':r['name'],'seconds':r['seconds'],**r['resources']} for r in w['calls'] if 'resources' in r)
write('resources.json',{'scope':'Same named-UV and morph rendering settings with all eight UV sets; process observations are not benchmark comparisons or allocator guarantees','observations':observations})
evidence.mkdir(exist_ok=False)
for p in root.glob('*.json'):shutil.copy2(p,evidence/p.name)
for folder in ['logs','source-validation','named-uv-regression','workflow']:shutil.copytree(root/folder,evidence/folder)
for folder in ['animated-regression','shutter-regression','sparse-regression','alpha-regression','camera-regression']:
    (evidence/folder).mkdir();name='camera_report.json' if folder=='camera-regression' else 'workflow_report.json';shutil.copy2(root/folder/name,evidence/folder/name)
for folder in ['source-audit','source-audit-initial']:shutil.copytree(root.parent/folder,evidence/folder)
shutil.copy2('/private/tmp/render-gltf-validator-20260906/package-lock.json',evidence/'validator-package-lock.json')
for p in evidence.rglob('*.txt'):
    s=p.read_text();p.write_text('\n'.join(line.rstrip() for line in s.splitlines()).rstrip()+'\n' if s.strip() else '')
print(json.dumps({'unchanged_artifacts':len(identical),'identity_only_artifacts':len(corrected),'fixture_files':len(fixtures),'corrected_files':sum(not r['unchanged'] for r in fixtures)}))
