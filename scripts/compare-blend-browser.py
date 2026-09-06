"""Compare final native/Wasm CPU and GPU products with explicit numeric limits."""
import json,math,struct,sys
from pathlib import Path
root=Path(sys.argv[1]);workflow=json.loads((root/'workflow_report.json').read_text());rows=[]
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
for variant in workflow['variants']:
 name=variant['name'];native=pfm(root/(name+'-cpu/image/image.pfm'));metal=pfm(root/(name+'-gpu/image/image.pfm'));cpu=json.loads((root/('browser-'+name+'-cpu.json')).read_text());gpu=json.loads((root/('browser-'+name+'-gpu.json')).read_text());a=[x for rgb in cpu['linear_rgb'] for x in rgb];b=[x for rgb in gpu['linear_rgb'] for x in rgb]
 assert a==native,(name,'native/Wasm CPU pixels differ')
 rmse=math.sqrt(sum((x-y)**2 for x,y in zip(native,b,strict=True))/len(native));assert rmse<5e-5,(name,rmse)
 assert gpu['object_ids']==cpu['object_ids'],(name,'GPU object pass')
 assert max(abs(x-y) for x,y in zip(metal,b,strict=True))<2e-5,(name,'Metal/WebGPU pixels')
 rows.append({'name':name,'native_wasm_cpu_pixels_identical':True,'browser_cpu_gpu_rmse':rmse,'native_browser_gpu_max_channel_error':max(abs(x-y) for x,y in zip(metal,b,strict=True)),'object_pass_identical':True})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','cases':rows},indent=2)+'\n');print(json.dumps(rows))
