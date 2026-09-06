"""Resume immutable Rust acceptance after an independently recorded harness-only fix."""
import hashlib,json
from pathlib import Path
root=Path('artifacts/tiled-painting/run-20260906T200434Z')
dev=Path('artifacts/tiled-painting/development')
original=json.loads((dev/'before-browser-resume-validation_report.json').read_text())
integrity=json.loads((dev/'browser-resume-integrity.json').read_text())
for row in integrity['packages']:assert hashlib.sha256(Path(row['path']).read_bytes()).hexdigest()==row['sha256'],row['path']
for row in json.loads((root/'source_manifest.json').read_text()):
 p=Path(row['path'])
 if p.suffix=='.rs' or p.name in ['Cargo.toml','Cargo.lock']:assert hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256'],p
source=Path('scripts/validate-tiled-painting.py').read_text()
start=source.index("root=Path('artifacts/tiled-painting')");end=source.index('\n',start)
source=source[:start]+f'root=Path({str(root)!r})'+source[end:]
source=source.replace("logs.mkdir(parents=True)","logs.mkdir(parents=True,exist_ok=True)")
start=source.index("report={'status':");end=source.index('\n',start)
kept=[c for c in original['checks'] if c['name'] in integrity['reused_checks']]
report=dict(status='in_progress',scope=original['scope'],checks=kept,harness_resume='See development browser-resume-integrity and before-browser-resume reports; unchanged compiled inputs/packages, native painting and browser rerun')
source=source[:start]+f'report={report!r}'+source[end:]
source=source.replace("(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\\n');print(root,flush=True)","print(root,flush=True)")
source=source.replace('for name,cmd in checks:\n',"for name,cmd in checks:\n    if name in {c['name'] for c in report['checks']}: continue\n",1)
exec(compile(source,'scripts/validate-tiled-painting.py','exec'),{})
