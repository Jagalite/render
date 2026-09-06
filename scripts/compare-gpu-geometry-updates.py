"""Compare the exact tested native/browser GPU-cache workflows and resource counts."""
import json,math,struct,sys
from pathlib import Path
root=Path(sys.argv[1])
def pfm(p):
 magic,size,endian,data=p.read_bytes().split(b'\n',3);w,h=map(int,size.split());assert magic==b'PF' and endian==b'-1.0';v=struct.unpack('<'+'f'*(w*h*3),data)
 return [x for y in range(h-1,-1,-1) for x in v[y*w*3:(y+1)*w*3]]
native=json.loads((root/'native_report.json').read_text());browser=json.loads((root/'browser_report.json').read_text());assert native['status']==browser['status']=='passed';rows=[]
for a,b in zip(native['cases'],browser['cases'],strict=True):
 name=a['name'];assert b['name']==name and a['revision']==b['revision'];assert a['statistics']==b['statistics'],(name,'upload decisions differ')
 cpu=pfm(root/name/'cpu.pfm');metal=pfm(root/name/'image.pfm');passes=json.loads((root/name/'passes.json').read_text())
 for backend in ['cpu','gpu']:
  image=json.loads((root/name/('browser-'+backend+'.json')).read_text());pixels=[x for p in image['linear_rgb'] for x in p]
  if backend=='cpu':assert cpu==pixels,(name,'native/Wasm CPU differs')
  rmse=math.sqrt(sum((x-y)**2 for x,y in zip(cpu,pixels,strict=True))/len(cpu));assert rmse<.002,(name,backend,rmse)
  metal_error=max(abs(x-y) for x,y in zip(metal,pixels,strict=True))
  if backend=='gpu':assert metal_error<2e-5,(name,metal_error)
  depth=max(abs(x-y) for x,y in zip(passes[2],image['depth_meters'],strict=True));normal=max(abs(x-y) for p,q in zip(passes[3],image['normals_world'],strict=True) for x,y in zip(p,q,strict=True));assert depth<2e-5 and normal<2e-5 and passes[4]==image['object_ids'],(name,backend,depth,normal)
  rows.append(dict(name=name,backend=backend,cpu_rmse=rmse,metal_max_error=metal_error,depth_max_error=depth,normal_max_error=normal,object_ids_identical=True))
assert len(browser['invalid'])==6
report=dict(status='passed',profile='gpu-buffer-updates-v1',native_wasm_cpu_exact=True,upload_decisions_and_counts_identical=True,comparisons=rows)
(root/'cross_platform.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
