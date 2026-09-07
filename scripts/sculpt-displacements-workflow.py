"""Native sparse displacement authoring, rendering and archive recovery."""
import copy,hashlib,json,os,re,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
binary=Path('target/debug/render-host').resolve()
report=dict(status='in_progress',calls=[],edits=[])
def save(): (root/'workflow_report.json').write_text(json.dumps(report,indent=2)+'\n')
def call(name,op,target=None,error=None,limits=None,resource=False):
 target=target or root/'project';wire=dict(version=0,operation=op)
 if limits is not None:wire['limits']=limits
 path=root/(f'{len(report["calls"]):03}-{name}.request.json');path.write_text(json.dumps(wire,indent=2)+'\n')
 command=[str(binary),'project',str(target),str(path)]
 if resource:command=['/usr/bin/time','-l']+command
 start=time.monotonic();result=subprocess.run(command,text=True,capture_output=True,timeout=120)
 path.with_suffix('.stdout.json').write_text(result.stdout);path.with_suffix('.stderr.txt').write_text(result.stderr)
 row=dict(name=name,command=command,exit_code=result.returncode,seconds=time.monotonic()-start,expected_error=error)
 if resource and result.returncode==0:row['maximum_resident_bytes']=int(re.search(r'(\d+)\s+maximum resident set size',result.stderr).group(1))
 report['calls'].append(row);save()
 if error:
  assert result.returncode!=0 and not result.stdout,(name,result)
  value=json.loads(result.stderr);assert value['code']==error,(name,value)
 else:
  assert result.returncode==0,(name,result.stderr);value=json.loads(result.stdout)
 print(name+': passed',flush=True);return value

def inspect(name,target=None):return call(name,dict(method='inspect'),target)
def apply(name,state,commands):
 request=dict(version=0,base_revision=state['revision'],idempotency_key='sculpt-workflow-'+name,max_added_bytes=4<<20,commands=commands)
 result=call(name,dict(method='apply',request=request));return request,result
identity=dict(columns=[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],operations=[])
def entity(n,mesh,material):return dict(id=f'{n:032x}',name=f'Sculpt fixture {n}',parent=None,mesh=mesh,material=f'{material:032x}',transform=identity)
try:
 report['tested_native']=dict(path=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),bytes=binary.stat().st_size)
 fixture=json.loads(Path('fixtures/sculpt-displacements/data/grid.json').read_text());doc_id=f'{16001:032x}'
 call('init',dict(method='init',document_id=doc_id));state=inspect('empty')
 apply('mesh',state,[dict(operation='put_mesh',mesh=fixture['mesh'])]);state=inspect('mesh');mesh=next(iter(state['snapshot']['meshes']))
 asset=dict(version=0,base_mesh=mesh,blocks={});apply('asset',state,[dict(operation='put_sculpt_asset',asset=asset)]);state=inspect('asset');source=next(iter(state['snapshot']['sculpt_assets']))
 material=dict(id=f'{2:032x}',base_color=[0.6,0.7,0.8],emission=[0,0,0],roughness=1,metallic=0,texture=None)
 settings=json.loads(Path('fixtures/uv-authoring/data/create.json').read_text())[-1]['settings'];settings.update(width=64,height=64,samples=4,max_depth=1);settings['camera'].update(position=[0,0,4],target=[0,0,0])
 apply('bind',state,[dict(operation='put_material',material=material),dict(operation='create_entity',entity=entity(1,mesh,2)),dict(operation='set_sculpt',entity=f'{1:032x}',source_mesh=mesh,source_asset=None,asset=source),dict(operation='set_render_settings',settings=settings)])
 state=inspect('initial');report.update(document_id=doc_id,initial_revision=state['revision'],source=source,mesh=mesh)
 call('initial-export',dict(method='export',revision=state['revision'],output=str(root/'initial-archive'),content=dict(format='document')))
 call('initial-render',dict(method='render',revision=state['revision'],backend='cpu',output=str(root/'initial-cpu')))
 budget=dict(max_point_updates=4096,max_changed_chunks=64,max_chunk_bytes=1<<20,max_basis_bytes=64<<20,max_triangle_updates=131072,max_refit_nodes=262144,max_copy_bytes=64<<20)
 q=dict(version=0,base_revision=state['revision'],idempotency_key='sculpt-workflow-author-001',entity=f'{1:032x}',source_asset=source,points=fixture['points'],budget=budget,max_added_bytes=1<<20)
 for name,change,code in [('stale',dict(base_revision='sha256:'+'0'*64),'stale_revision'),('duplicate',dict(points=fixture['points']*2),'sculpt_point'),('unknown',dict(points=[dict(point='1',delta_meters=[0,0,1])]),'sculpt_point'),('copy-budget',dict(budget=dict(budget,max_copy_bytes=1)),'budget')]:
  call(name,dict(method='author_sculpt',request=dict(q,**change)),error=code)
 cancel=root/'cancel';cancel.write_text('cancel');call('cancel',dict(method='author_sculpt',request=q),error='cancelled',limits=dict(cancel_file=str(cancel)))
 assert inspect('rejections-atomic')['document_digest']==state['document_digest']
 out=call('author',dict(method='author_sculpt',request=q),resource=True);report['edits'].append(dict(request=q,result=out));call('retry',dict(method='author_sculpt',request=q));state=inspect('changed');report['revision']=state['revision']
 call('archive',dict(method='export',revision=state['revision'],output=str(root/'archive'),content=dict(format='document')))
 for backend in ['cpu','gpu']:call('changed-'+backend,dict(method='render',revision=state['revision'],backend=backend,output=str(root/('changed-'+backend))),resource=True)
 assert (root/'initial-cpu/image/image.pfm').read_bytes()!=(root/'changed-cpu/image/image.pfm').read_bytes()
 restored=root/'restored';call('restore',dict(method='restore',source=str(root/'archive/document/document.json')),restored);assert inspect('restored',restored)['snapshot']==state['snapshot']
 call('restored-render',dict(method='render',revision=state['revision'],backend='cpu',output=str(root/'restored-cpu')),restored)
 assert (root/'changed-cpu/image/image.pfm').read_bytes()==(root/'restored-cpu/image/image.pfm').read_bytes()
 undo=dict(q,base_revision=state['revision'],idempotency_key='sculpt-workflow-clear-002',source_asset=out['report']['output'],points=[dict(point=p['point'],delta_meters=[0,0,0]) for p in q['points']])
 cleared=call('clear',dict(method='author_sculpt',request=undo));assert cleared['report']['output']==source;report['undo']=dict(request=undo,result=cleared)
 baseline=Path('artifacts/sculpt-displacements/baseline/render-host').resolve()
 oldwire=root/'old-client-inspect.json';oldwire.write_text(json.dumps(dict(version=0,operation=dict(method='inspect'))))
 old=subprocess.run([str(baseline),'project',str(root/'project'),str(oldwire)],capture_output=True,text=True)
 (root/'old-client.stdout.txt').write_text(old.stdout);(root/'old-client.stderr.txt').write_text(old.stderr)
 assert old.returncode!=0 and not old.stdout,('old client accepted snapshot19',old.stdout,old.stderr)
 report['old_client']=dict(sha256=hashlib.sha256(baseline.read_bytes()).hexdigest(),exit_code=old.returncode,error=old.stderr)
 state=inspect('cleared');call('clear-render',dict(method='render',revision=state['revision'],backend='cpu',output=str(root/'clear-cpu')))
 assert (root/'initial-cpu/image/image.pfm').read_bytes()==(root/'clear-cpu/image/image.pfm').read_bytes()
 report.update(status='passed',archive_recovery=True,restored_render_identical=True,clear_render_identical=True);save()
except BaseException as error:report.update(status='failed',error=str(error));save();raise
