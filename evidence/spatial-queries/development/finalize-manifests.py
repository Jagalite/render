import hashlib,json,subprocess
from pathlib import Path
root=Path('evidence/spatial-queries');assert json.loads((root/'collection.json').read_text())['status']=='passed'
def row(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
source=[row(Path(n)) for n in sorted(set(paths)) if Path(n).is_file() and not n.startswith(('artifacts/','evidence/','scripts/__pycache__/'))]
(root/'implementation_manifest.json').write_text(json.dumps({'files':source,'scope':'Final maintained sources, contracts, schemas, fixtures and documentation; generated packages and evidence have separate manifests'},indent=2)+'\n')
files=[row(p) for p in sorted(root.rglob('*')) if p.is_file() and p.name!='evidence_manifest.json']
(root/'evidence_manifest.json').write_text(json.dumps({'files':files,'bytes':sum(r['bytes'] for r in files)},indent=2)+'\n')
for manifest in ['implementation_manifest.json','evidence_manifest.json']:
 for entry in json.loads((root/manifest).read_text())['files']:
  p=Path(entry['path']);assert row(p)==entry,p
print(json.dumps({'source_files':len(source),'evidence_files':len(files),'evidence_bytes':sum(r['bytes'] for r in files)}))
