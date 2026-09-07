"""Independent wire-shape checks; semantic identity and geometry remain Rust gates."""
import copy,json,sys
from pathlib import Path
import jsonschema
root=Path(sys.argv[1]);report=json.loads((root/'workflow_report.json').read_text())
wire=jsonschema.Draft202012Validator(json.loads(Path('schemas/spatial_query.schema.json').read_text()))
result_validator=jsonschema.Draft202012Validator(json.loads(Path('schemas/spatial_query_result.schema.json').read_text()))
agent=jsonschema.Draft202012Validator(json.loads(Path('schemas/agent_request.schema.json').read_text()))
for row in report['queries']:
 result_validator.validate(row['result']);wire.validate(row['request']);agent.validate(dict(version=0,operation=dict(method='query_geometry',request=row['request'])))
negative=[]
for name,mutate in [
 ('version',lambda q:q.update(version=1)),('unknown',lambda q:q.update(runtime_index=0)),
 ('mesh',lambda q:q.update(mesh='missing')),('stale-shape',lambda q:q.update(base_revision='stale')),
 ('zero-budget',lambda q:q['budget'].update(max_item_tests=0)),('over-budget',lambda q:q['budget'].update(max_build_bytes=(32<<20)+1)),
 ('triangulation-cap',lambda q:q['budget'].update(max_triangulation_work=16777217)),
 ('radius',lambda q:q['query'].update(max_distance_meters=-1)),('point',lambda q:q['query'].update(point=[0,0])),
 ('metric-cap',lambda q:q['query'].update(point=[1e9+1,0,0])),('unknown-query',lambda q:q['query'].update(space='world'))]:
 q=copy.deepcopy(report['queries'][0]['request']);mutate(q);assert list(wire.iter_errors(q)),name;negative.append(name)
result=dict(status='passed',positive=len(report['queries']),negative=negative)
(root/'schema_validation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
