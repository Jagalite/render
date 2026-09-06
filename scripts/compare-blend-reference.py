"""Independent numeric facts for the pinned official startup data; no Blender runtime."""
import json,math,sys
from pathlib import Path
root=Path(sys.argv[1]);rows=json.loads((root/'workflow_report.json').read_text())['variants'];row=next((x for x in rows if x['name']=='startup-293'),None)
if row is None:raise ValueError('Pinned real reference is required for this gate')
assert row['report']['source_digest']=='sha256:339bc1c5cdc7fb0f3a9250f66d9616b4511e42bf08feb31ccf01344b5db655ea'
mesh=row['native_mesh'];expected=[[1,1,1],[1,1,-1],[1,-1,1],[1,-1,-1],[-1,1,1],[-1,1,-1],[-1,-1,1],[-1,-1,-1]]
assert mesh['positions']['values']==expected;assert mesh['face_offsets']==[0,4,8,12,16,20,24]
assert [x['vertex'] for x in mesh['corners']]==[0,4,6,2,3,2,6,7,7,6,4,5,5,1,3,7,1,0,2,3,5,4,0,1]
assert len(mesh['edges'])==12 and len(mesh['attributes']['UVMap']['values']['values'])==24
entities={x['name']:x for x in row['native_entities']};assert set(entities)=={'Camera','Cube','Light'}
positions={'Camera':[7.358891487121582,-6.925790786743164,4.958309173583984],'Cube':[0,0,0],'Light':[4.076245307922363,1.0054539442062378,5.903861999511719]}
for name,e in entities.items():assert e['parent'] is None;assert e['transform']['operations'][0]=={'kind':'TranslationMeters','value':positions[name]}
# Independent elementary XYZ matrix formula, compared with camera world axes.
x,z=1.1093189716339111,0.8149281740188599
sx,cx,sz,cz=math.sin(x),math.cos(x),math.sin(z),math.cos(z)
expected_up=[-sz*cx,cz*cx,sx];expected_z=[sz*sx,-cz*sx,cx]
s=row['native_settings'];camera=s['camera'];expected_target=[p-v for p,v in zip(positions['Camera'],expected_z)]
errors=[abs(a-b) for a,b in zip(camera['up'],expected_up)]+[abs(a-b) for a,b in zip(camera['target'],expected_target)]
assert max(errors)<2e-14;fov=2*math.atan(36/(2*50)/(1920/1080));assert abs(camera['lens']['vertical_fov_radians']-fov)<2e-15
assert s['light']['position']==positions['Light'];assert max(abs(c-1000/(4*math.pi)) for c in s['light']['intensity'])<4e-6
assert s['environment']==[0.05087608844041824]*3
report={'status':'passed','source_digest':row['report']['source_digest'],'geometry':'8 vertices,12 edges,6 quads,24 corners and named UVMap','authored_object_positions_exact':True,'camera_xyz_formula_max_absolute_error':max(errors),'camera_vertical_fov_radians':fov,'point_power':1000,'point_radius_loss_meters':3,'source_asset_json_bytes':row['report']['source_asset_json_bytes'],'source_bytes':row['report']['source_bytes'],'reference_method':'Pinned binary data values inspected independently; elementary camera and isotropic-light formulas; no source implementation code reused'}
(root/'reference_comparison.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
