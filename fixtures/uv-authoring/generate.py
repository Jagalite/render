"""Original CC0 UV authoring inputs; does not generate expected render outputs."""
import hashlib, json, sys
from pathlib import Path
out=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent/'data'
out.mkdir(parents=True,exist_ok=True)
def ident(n):return f'{n:032x}'
settings=json.loads((Path(__file__).resolve().parents[1]/'project-cli/create.json').read_text())[-1]['settings']
settings.update(width=32,height=32,samples=8,max_depth=2,environment=[.1,.1,.1])
settings['camera'].update(position=[3,2,4],target=[0,0,0],up=[0,1,0])
settings['light'].update(position=[2,4,3],intensity=[32,32,32])
material={'id':ident(2),'base_color':[.8,.8,.8],'emission':[0,0,0],'roughness':1,'metallic':0,'texture':{'width':8,'height':8,'linear_rgb':[[1,1,1] if (x+y)%2 else [.1,.2,.8] for y in range(8) for x in range(8)]}}
commands=[{'operation':'put_material','material':material},{'operation':'create_entity','entity':{'id':ident(1),'name':'UV authored checker box','parent':None,'mesh':None,'material':ident(2),'transform':{'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}}},{'operation':'create_box','entity':ident(1),'min':[-.5,-.5,-.5],'max':[.5,.5,.5]},{'operation':'set_render_settings','settings':settings}]
files={'create.json':commands,'unwrap.json':{'attribute':'PaintUV','attribute_id':ident(500),'seams':list(range(1,13)),'pins':[]},'pack.json':{'atlas':{'width':64,'height':64,'padding_pixels':1,'pixels_per_meter':12},'pins':'preserve'}}
files['padding2-no-fit.json']={'atlas':{'width':64,'height':64,'padding_pixels':2,'pixels_per_meter':12},'pins':'preserve'}
rows=[]
for name,value in files.items():
 data=(json.dumps(value,indent=2)+'\n').encode();(out/name).write_bytes(data);rows.append({'file':name,'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
(out/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','author':'Render project original cube and checker inputs','generator':'fixtures/uv-authoring/generate.py','files':rows},indent=2)+'\n')
