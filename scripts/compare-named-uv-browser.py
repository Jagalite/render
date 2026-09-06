"""Compare saved native and browser passes for the original UV-role oracle."""
import json,struct,sys
from pathlib import Path
root=Path(sys.argv[1]);output=Path(sys.argv[2]);p=root/'roles-cpu/image'
h,d,e,b=(p/'image.pfm').read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0'
v=struct.unpack('<'+'f'*(w*y*3),b);native=[x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
passes=json.loads((p/'passes.json').read_text());comparisons=[]
for backend in ['cpu','gpu']:
    browser=json.loads((root/f'browser-roles-{backend}.json').read_text());rgb=[x for p in browser['linear_rgb'] for x in p]
    rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
    depth=max(abs(a-b) for a,b in zip(passes[2],browser['depth_meters'],strict=True))
    normals=max(abs(a-b) for a,b in zip((x for p in passes[3] for x in p),(x for p in browser['normals_world'] for x in p),strict=True))
    assert passes[4]==browser['object_ids'] and rmse<.002 and depth<2e-5 and normals<.002
    if backend=='cpu':assert rmse==depth==normals==0
    comparisons.append({'backend':backend,'linear_rmse':rmse,'max_depth_error':depth,'max_normal_error':normals,'object_mismatches':0})
output.write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n');print(output.read_text())
