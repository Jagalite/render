"""Generate original bounded indirect-light fixture commands into a NEW directory."""
import json,sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=False)
id=lambda n:f'{n:032x}'
commands=[]
for n,lo,hi,color,emission,pbr in [
 (1,[-20,-20,-.1],[20,20,0],[.8,.5,.25],[0,0,0],True),
 (2,[-20,-20,2],[20,20,2.1],[.5,.5,.5],[1,.8,.5],True),
 (3,[-20,-20,0],[-19,20,2],[.5,.2,.2],[0,0,0],False),
 (4,[19,-20,0],[20,20,2],[.2,.5,.2],[0,0,0],False),
]:
 material={'id':id(100+n),'base_color':color,'emission':emission,'roughness':1,'metallic':.5 if n==1 else 0,'texture':None}
 if pbr:material['pbr']={'double_sided':False,'base_color':None,'metallic_roughness':None,'normal':None,'emission':None,'occlusion':None,'normal_scale':1,'occlusion_strength':1}
 commands.extend([{'operation':'put_material','material':material},{'operation':'create_entity','entity':{'id':id(n),'name':'plate '+str(n),'parent':None,'mesh':None,'material':id(100+n),'transform':{'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}}},{'operation':'create_box','entity':id(n),'min':lo,'max':hi}])
commands.append({'operation':'set_render_settings','settings':{'width':16,'height':16,'samples':64,'seed':73,'max_depth':4,'max_bytes':16777216,'environment':[0,0,0],'light':{'position':[0,0,1],'intensity':[0,0,0]},'camera':{'position':[0,0,1],'target':[0,0,0],'up':[0,1,0],'vertical_fov_radians':.5,'lens':{'kind':'orthographic','xmag':1,'ymag':1,'near':.01,'far':1.5}}}})
(root/'create.json').write_text(json.dumps(commands,indent=2)+'\n')
