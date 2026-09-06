"""Independent JSON Schema admission checks for the public UV request envelope."""
import copy,json,sys
from pathlib import Path
import jsonschema
root=Path(sys.argv[1]);schema=json.loads(Path('schemas/agent_request.schema.json').read_text());validator=jsonschema.Draft202012Validator(schema)
workflow=json.loads((root/'workflow_report.json').read_text());positive=[];negative=[]
for step in workflow['operations']:
 request={'version':0,'operation':step['operation']};validator.validate(request);positive.append(request)
for name,mutate in [
 ('unknown-field',lambda r:r['operation']['request'].update(runtime_index=0)),
 ('unknown-method',lambda r:r['operation'].update(method='uv_private')),
 ('future-version',lambda r:r['operation']['request'].update(version=1)),
 ('bad-source-hash',lambda r:r['operation']['request'].update(source_mesh='missing')),
 ('private-identity',lambda r:r['operation']['request'].update(entity=0)),
 ('zero-budget',lambda r:r['operation']['request']['budget'].update(max_solver_work=0)),
 ('excess-work',lambda r:r['operation']['request']['budget'].update(max_solver_work=16777217)),
 ('bad-attribute',lambda r:r['operation']['request']['operation']['settings'].update(attribute='')),
 ('pin-shape',lambda r:r['operation']['request']['operation']['settings'].update(pins=[{'corner':1,'uv':[0]}])),
 ('pin-range',lambda r:r['operation']['request']['operation']['settings'].update(pins=[{'corner':1,'uv':[1025,0]}])),
 ('duplicate-seam',lambda r:r['operation']['request']['operation']['settings'].update(seams=[1,1])),
]:
 r=copy.deepcopy(positive[0]);mutate(r);assert list(validator.iter_errors(r)),name;negative.append(name)
for name,mutate in [
 ('pin-policy',lambda r:r['operation']['request']['operation']['settings'].update(pins='silently_move')),
 ('zero-atlas',lambda r:r['operation']['request']['operation']['settings']['atlas'].update(width=0)),
 ('negative-density',lambda r:r['operation']['request']['operation']['settings']['atlas'].update(pixels_per_meter=-1)),
]:
 r=copy.deepcopy(positive[1]);mutate(r);assert list(validator.iter_errors(r)),name;negative.append(name)
report={'status':'passed','positive':len(positive),'negative':negative,'boundary':'shape only; Rust enforces UTF-8 byte counts, finite numbers, identities, topology and atomic publication'}
(root/'schema_validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
