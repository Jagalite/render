"""Compare authored identities and the exact tested native/browser render products."""
import json,math,struct,sys
from pathlib import Path
root=Path(sys.argv[1])
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
a=pfm(root/'box-cpu/image/image.pfm');metal=pfm(root/'box-gpu/image/image.pfm');cpu=json.loads((root/'browser-box-cpu.json').read_text());gpu=json.loads((root/'browser-box-gpu.json').read_text())
b=[x for p in cpu['linear_rgb'] for x in p];c=[x for p in gpu['linear_rgb'] for x in p]
assert a==b,'native/Wasm CPU pixels differ'
rmse=math.sqrt(sum((x-y)**2 for x,y in zip(a,c,strict=True))/len(a));assert rmse<.002,rmse
error=max(abs(x-y) for x,y in zip(metal,c,strict=True));assert error<2e-5,error
passes=json.loads((root/'box-cpu/image/passes.json').read_text());assert passes[2]==cpu['depth_meters'] and passes[3]==cpu['normals_world'] and passes[4]==cpu['object_ids'];assert cpu['object_ids']==gpu['object_ids']
metal_passes=json.loads((root/'box-gpu/image/passes.json').read_text());gpu_pass_comparisons=[]
for backend,depth,normals,objects in [('metal',metal_passes[2],metal_passes[3],metal_passes[4]),('webgpu',gpu['depth_meters'],gpu['normals_world'],gpu['object_ids'])]:
 depth_error=max(abs(x-y) for x,y in zip(passes[2],depth,strict=True))
 normal_error=max(abs(x-y) for p,q in zip(passes[3],normals,strict=True) for x,y in zip(p,q,strict=True))
 assert depth_error<2e-5 and normal_error<.002 and passes[4]==objects,(backend,depth_error,normal_error)
 gpu_pass_comparisons.append(dict(backend=backend,depth_max_error=depth_error,normal_max_error=normal_error,object_ids_identical=True))
workflow=json.loads((root/'workflow_report.json').read_text());browser=json.loads((root/'browser_report.json').read_text());assert workflow['revision']==browser['revision']
for a,b in zip(workflow['operations'],browser['results'],strict=True):
 assert a['result']['receipt']['revision']==b['receipt']['revision']
 if 'report' in a['result']:assert a['result']['report']==b['report'],'native/Wasm numeric diagnostics differ'
report={'status':'passed','native_wasm_tiles_images_revisions_reports_identical':True,'native_wasm_cpu_pixels_passes_identical':True,'browser_cpu_gpu_rmse':rmse,'metal_webgpu_max_channel_error':error,'gpu_pass_comparisons':gpu_pass_comparisons}
(root/'cross_platform.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
