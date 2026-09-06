"""Independent numeric comparison of native Metal and browser WebGPU pass files."""
import json
from pathlib import Path
import struct
import sys
root=Path('artifacts/static-pbr');native=Path(sys.argv[1]);rows=[]
rmse=lambda a,b:(sum((x-y)**2 for x,y in zip(a,b,strict=True))/len(a))**0.5
flat=lambda a:[x for row in a for x in row]
for name in ['BoxTextured','NormalTangentMirrorTest']:
    header,dims,endian,payload=(native/(name+'-gpu.pfm')).read_bytes().split(b'\n',3)
    assert header==b'PF' and endian==b'-1.0'
    width,height=map(int,dims.split());values=struct.unpack('<'+'f'*(width*height*3),payload)
    rgb=[x for row in range(height-1,-1,-1) for x in values[row*width*3:(row+1)*width*3]]
    nw,nh,depth,normals,objects=json.loads((native/(name+'-gpu.passes.json')).read_text());assert (nw,nh)==(width,height)
    browser=json.loads((root/(name+'-browser-passes.json')).read_text())
    row={'asset':name,'linear_rmse':rmse(rgb,flat(browser['linear_rgb'])),'normal_rmse':rmse(flat(normals),flat(browser['normals_world'])),'depth_rmse':rmse(depth,browser['depth_meters']),'object_mismatches':sum(x!=y for x,y in zip(objects,browser['object_ids'],strict=True))}
    assert row['linear_rmse']<0.025 and row['normal_rmse']<0.025 and row['depth_rmse']<0.0001 and row['object_mismatches']<=4,row
    rows.append(row)
(root/'cross_platform_report.json').write_text(json.dumps({'status':'passed','native_directory':str(native),'assets':rows},indent=2)+'\n')
print(json.dumps(rows))
