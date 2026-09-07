"""Native read-only spatial queries, ordinary placement, archive/recovery and costs."""
import copy,hashlib,json,os,re,subprocess,sys,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();root.mkdir(parents=True,exist_ok=False)
binary=Path('target/debug/render-host').resolve()
report=dict(status='in_progress',calls=[],queries=[])
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
 request=dict(version=0,base_revision=state['revision'],idempotency_key='spatial-workflow-'+name,max_added_bytes=4<<20,commands=commands)
 result=call(name,dict(method='apply',request=request));return request,result
identity=dict(columns=[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],operations=[])
def entity(n,mesh,material):return dict(id=f'{n:032x}',name=f'Spatial fixture {n}',parent=None,mesh=mesh,material=f'{material:032x}',transform=identity)
try:
 report['tested_native']=dict(path=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),bytes=binary.stat().st_size)
 fixture=json.loads(Path('fixtures/spatial-queries/data/high-id-grid.json').read_text());doc_id=f'{980:032x}'
 call('init',dict(method='init',document_id=doc_id));state=inspect('empty')
 apply('source',state,[dict(operation='put_mesh',mesh=fixture['mesh']),dict(operation='put_mesh',mesh=fixture['tiny']),dict(operation='put_mesh',mesh=fixture['conditioned'])]);state=inspect('source');source=next(k for k,m in state['snapshot']['meshes'].items() if len(m['point_ids'])==291);tiny=next(k for k,m in state['snapshot']['meshes'].items() if len(m['point_ids'])==3);conditioned=next(k for k,m in state['snapshot']['meshes'].items() if len(m['point_ids'])==4)
 material=dict(id=f'{2:032x}',base_color=[0.6,0.7,0.8],emission=[0,0,0],roughness=1,metallic=0,texture=None)
 marker=dict(material,id=f'{3:032x}',base_color=[0.9,0.2,0.1])
 settings=json.loads(Path('fixtures/uv-authoring/data/create.json').read_text())[-1]['settings'];settings.update(width=16,height=16,samples=4)
 settings['camera'].update(position=[0,0,7],target=[0,0,0]);settings['max_depth']=1
 commands=[dict(operation='put_material',material=material),dict(operation='put_material',material=marker),dict(operation='create_entity',entity=entity(1,source,2)),dict(operation='create_entity',entity=entity(4,None,3)),dict(operation='create_box',entity=f'{4:032x}',min=[-.1,-.1,0],max=[.1,.1,.2]),dict(operation='set_render_settings',settings=settings)]
 apply('scene',state,commands);state=inspect('scene');revision=state['revision'];before=state['document_digest']
 call('initial-export',dict(method='export',revision=revision,output=str(root/'initial-archive'),content=dict(format='document')))
 budget=dict(max_source_bytes=8<<20,max_build_bytes=32<<20,max_triangulation_work=16777216,max_visited_nodes=262144,max_item_tests=131072,max_hits=4096,max_result_bytes=1<<20)
 query_inputs=[(source,q) for q in fixture['queries']]+[(tiny,q) for q in fixture['tiny_queries']]+[(source,fixture['queries'][0])]
 for i,(mesh,query) in enumerate(query_inputs):
  q=dict(version=0,base_revision=revision,mesh=mesh,query=query,budget=budget)
  result=call(f'query-{i}',dict(method='query_geometry',request=q),resource=True)
  assert not result['cost']['cache_reused'];report['queries'].append(dict(request=q,result=result));save()
 hit=report['queries'][0]['result']['hits']['surface'];assert hit['position_meters']==[.625,.375,0.] and hit['distance_meters']==1.
 assert int(hit['face'])>2**53 and all(int(c)>2**53 for c in hit['corners'])
 zero=report['queries'][1]['result']['hits']['points'];assert len(zero)==3 and all(p['distance_meters']==0 for p in zero)
 assert [int(p['point']) for p in zero]==sorted(int(p['point']) for p in zero)
 tiny_hit=report['queries'][5]['result']['hits']['surface'];assert tiny_hit['geometric_normal']==[0,0,1]
 assert max(abs(a-b) for a,b in zip(tiny_hit['barycentric'],[.5,.25,.25]))<1e-12
 assert abs(tiny_hit['distance_meters']/1e-81-1)<1e-12
 assert report['queries'][6]['result']['hits']['points']==[]
 q=report['queries'][0]['request']
 for name,bad,code in [('stale',dict(q,base_revision='sha256:'+'0'*64),'stale_revision'),('future',dict(q,version=1),'version'),('conditioned-frame',dict(q,mesh=conditioned),'degenerate'),('budget',dict(q,budget=dict(budget,max_item_tests=1)),'budget'),('triangulation-budget',dict(q,budget=dict(budget,max_triangulation_work=1)),'budget'),('metric',dict(q,query=dict(kind='nearest_surface',point=[0,0,0],max_distance_meters=-1)),'query_metric')]:call(name,dict(method='query_geometry',request=bad),error=code)
 cancel=root/'cancel';cancel.write_text('cancel');call('cancel',dict(method='query_geometry',request=q),error='cancelled',limits=dict(cancel_file=str(cancel)))
 assert inspect('queries-atomic')['document_digest']==before
 baseline=Path(os.environ['RENDER_SPATIAL_BASELINE_HOST']);oldwire=root/'old-query.json';oldwire.write_text(json.dumps(dict(version=0,operation=dict(method='query_geometry',request=q))))
 old=subprocess.run([str(baseline),'project',str(root/'project'),str(oldwire)],capture_output=True,text=True);(root/'old-query.stderr.json').write_text(old.stderr)
 assert old.returncode!=0 and not old.stdout and json.loads(old.stderr)['code']=='encoding'
 assert inspect('old-client-atomic')['document_digest']==before
 report['old_client']=dict(sha256=hashlib.sha256(baseline.read_bytes()).hexdigest(),error='encoding',unchanged=True)
 transform=copy.deepcopy(identity);transform['operations']=[dict(kind='TranslationMeters',value=hit['position_meters'])]
 placement,result=apply('place-marker',state,[dict(operation='set_transform',entity=f'{4:032x}',transform=transform)])
 report['placement']=dict(request=placement,result=result);state=inspect('placed');report.update(document_id=doc_id,initial_revision=revision,revision=state['revision'],source=source,conditioned_mesh=conditioned)
 call('final-export',dict(method='export',revision=state['revision'],output=str(root/'archive'),content=dict(format='document')))
 for backend in ['cpu','gpu']:call('render-'+backend,dict(method='render',revision=state['revision'],backend=backend,output=str(root/('placed-'+backend))))
 restored=root/'restored';call('restore',dict(method='restore',source=str(root/'archive/document/document.json')),restored)
 assert inspect('restored',restored)['snapshot']==state['snapshot']
 for i,row in enumerate(report['queries']):
  request=dict(row['request'],base_revision=state['revision']);result=call(f'restored-query-{i}',dict(method='query_geometry',request=request),restored)
  assert result['hits']==row['result']['hits']
 call('restored-render',dict(method='render',revision=state['revision'],backend='cpu',output=str(root/'placed-restored')),restored)
 assert (root/'placed-cpu/image/image.pfm').read_bytes()==(root/'placed-restored/image/image.pfm').read_bytes()
 report.update(status='passed',queries_read_only=True,archive_recovery=True,restored_render_identical=True);save()
except BaseException as error:report.update(status='failed',error=str(error));save();raise
