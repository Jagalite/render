"""Optional pinned development data fetch. Never imported by application code."""
import hashlib,json,subprocess,sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True)
commit='cb886aba06d562ee629f2ee64f3692d008c68a35'
base=f'https://raw.githubusercontent.com/blender/blender/{commit}/'
items=[('startup-293.blend','release/datafiles/startup.blend','339bc1c5cdc7fb0f3a9250f66d9616b4511e42bf08feb31ccf01344b5db655ea'),('COPYING','COPYING',None),('GPL-license.txt','doc/license/GPL-license.txt',None)]
report=[]
for name,path,expected in items:
 dst=root/name
 if not dst.exists():subprocess.run(['/usr/bin/curl','--fail','--location','--silent','--show-error','--max-time','30','--max-filesize','4194304',base+path,'--output',str(dst)],check=True)
 data=dst.read_bytes();sha=hashlib.sha256(data).hexdigest()
 if expected and sha!=expected:raise ValueError('Pinned fixture digest mismatch: '+name)
 report.append({'file':name,'source':base+path,'sha256':sha,'bytes':len(data)})
(root/'reference-provenance.json').write_text(json.dumps({'source_commit':commit,'redistribution':'Reference bytes and source-containing archives are not included in tracked evidence','files':report},indent=2)+'\n')
print(json.dumps(report))
