"""Validate recorded public authoring wires plus independent malformed examples."""
import copy,json,sys
from pathlib import Path
import jsonschema
root=Path(sys.argv[1]);schema=json.loads(Path('schemas/sculpt_displacements.schema.json').read_text());agent=json.loads(Path('schemas/agent_request.schema.json').read_text())
validator=jsonschema.Draft202012Validator(schema);jsonschema.Draft202012Validator.check_schema(schema)
report=json.loads((root/'workflow_report.json').read_text());requests=[r['request'] for r in report['edits']]+[report['undo']['request']]
for q in requests:
 validator.validate(q);jsonschema.validate(dict(version=0,operation=dict(method='author_sculpt',request=q)),agent)
q=requests[0];invalid=[]
for name,change in [('numeric-point',dict(points=[dict(point=1,delta_meters=[0,0,1])])),('leading-zero',dict(points=[dict(point='01',delta_meters=[0,0,1])])),('extra-field',dict(private_cache=0)),('wrong-version',dict(version=1)),('wrong-vector',dict(points=[dict(point='1',delta_meters=[0,1])]))]:
 bad=dict(q,**change);assert not validator.is_valid(bad),name;invalid.append(name)
result=dict(status='passed',valid_requests=len(requests),invalid=invalid,semantic_range_validation='Rust additionally checks exact u64/block range and finite f32 representability')
(root/'schema_validation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
