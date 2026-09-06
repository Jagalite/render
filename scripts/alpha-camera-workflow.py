"""Reproduce analytic alpha camera cases through ordinary native CLI requests."""
from pathlib import Path
import json,subprocess,datetime,hashlib,sys
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
project=root/'project';calls=[]
def call(name,op):
 req=root/(name+'.request.json');req.write_text(json.dumps({'version':0,'operation':op},indent=2)+'\n')
 command=['target/debug/render-host','project',str(project),str(req)];r=subprocess.run(command,capture_output=True,text=True);(root/(name+'.stdout.json')).write_text(r.stdout);(root/(name+'.stderr.txt')).write_text(r.stderr);assert r.returncode==0,r.stderr;calls.append(command);return json.loads(r.stdout)
id=lambda n:f'{n:032x}'
call('init',{'method':'init','document_id':id(9800)})
settings=json.loads(Path('fixtures/project-cli/create.json').read_text())[-1]['settings'];settings.update(width=8,height=8,samples=1,max_depth=1,environment=[0,0,0]);settings['light']['intensity']=[0,0,0]
settings['camera'].update(position=[0,0,3],target=[0,0,0],vertical_fov_radians=.6,lens={'kind':'perspective','vertical_fov_radians':.6,'aspect_ratio':1,'near':.1,'far':2.5})
commands=[]
for e,m,z,transparent in [(1,2,1,True),(3,4,0,False)]:
 material={'id':id(m),'base_color':[0,0,0],'emission':[0,0,0] if transparent else [1,0,0],'roughness':1,'metallic':0,'texture':None}
 if transparent: material['pbr']={'advanced':{'model':{'kind':'principled'},'opacity':{'kind':'mask','factor':0,'cutoff':.5}},'double_sided':True,'base_color':None,'metallic_roughness':None,'normal':None,'emission':None,'occlusion':None,'normal_scale':1,'occlusion_strength':1}
 transform={'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}
 commands.extend([{'operation':'put_material','material':material},{'operation':'create_entity','entity':{'id':id(e),'name':'transparent-front' if transparent else 'clipped-emitter','parent':None,'mesh':None,'material':id(m),'transform':transform}},{'operation':'create_box','entity':id(e),'min':[-10,-10,z-.1],'max':[10,10,z]}])
commands.append({'operation':'set_render_settings','settings':settings})
state=call('inspect-empty',{'method':'inspect'});call('author',{'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'alpha:baseline:01','max_added_bytes':8*1024*1024,'commands':commands}})
state=call('inspect',{'method':'inspect'});call('perspective',{'method':'render','revision':state['revision'],'backend':'cpu','output':str(root/'perspective')})
p=json.loads((root/'perspective/image/passes.json').read_text());visible=sum(x is not None for x in p[4])
settings['camera']['lens']={'kind':'orthographic','xmag':1,'ymag':1,'near':.1,'far':5}
call('orthographic-settings',{'method':'apply','request':{'version':0,'base_revision':state['revision'],'idempotency_key':'alpha:baseline:02','max_added_bytes':8*1024*1024,'commands':[{'operation':'set_render_settings','settings':settings}]}})
state=call('inspect-ortho',{'method':'inspect'});call('orthographic',{'method':'render','revision':state['revision'],'backend':'cpu','output':str(root/'orthographic')})
p=json.loads((root/'orthographic/image/passes.json').read_text());depth_error=max(abs(v-3) for v in p[2]);source=Path('crates/core/src/scattering.rs').read_bytes()
report={'source_checkpoint':'working alpha implementation','scattering_sha256':hashlib.sha256(source).hexdigest(),'perspective_expected_visible_pixels':0,'perspective_actual_visible_pixels':visible,'orthographic_expected_constant_depth':3,'orthographic_actual_depth_range':[min(p[2]),max(p[2])],'orthographic_max_error':depth_error,'commands':calls}
(root/'camera_report.json').write_text(json.dumps(report,indent=2)+'\n');print(root);print(json.dumps(report,indent=2))

assert visible==0,report
assert depth_error==0,report
