"""Development-only schema validation of actual export requests and malformed DTOs."""
import copy,hashlib,json,sys
from pathlib import Path
from jsonschema import Draft202012Validator
root=Path(sys.argv[1]);schema=Path('schemas/agent_request.schema.json');definition=json.loads(schema.read_text());Draft202012Validator.check_schema(definition);validator=Draft202012Validator(definition)
accepted=[]
for path in sorted((root/'workflow').glob('*.request.json')):
    value=json.loads(path.read_text());op=value['operation'];content=op.get('content',{})
    if op['method']!='export' or content.get('format')!='glb':continue
    request={'version':0,'operation':{'method':'export_glb','request':{'revision':op['revision'],'at':content.get('at'),'policy':content['policy']}}}
    validator.validate(request);accepted.append(path.name);base=request
negative=[]
for field,value in [('max_bytes',4194305),('max_vertices',0),('max_position_error_meters',0),('allow_approximations','yes'),('unknown',1)]:
    bad=copy.deepcopy(base);bad['operation']['request']['policy'][field]=value;assert list(validator.iter_errors(bad));negative.append(field)
for mutation in ['missing_revision','extra_request','zero_denominator','unknown_method']:
    bad=copy.deepcopy(base);q=bad['operation']['request']
    if mutation=='missing_revision':del q['revision']
    elif mutation=='extra_request':q['hidden_state']=True
    elif mutation=='zero_denominator':q['at']={'clip':'0'*32,'time':{'numerator':0,'denominator':0}}
    else:bad['operation']['method']='execute_script'
    assert list(validator.iter_errors(bad));negative.append(mutation)
assert accepted
result={'status':'passed','scope':'Structural JSON Schema subset; semantic consent, stale revisions and actual output budgets remain Rust checks','schema_sha256':hashlib.sha256(schema.read_bytes()).hexdigest(),'accepted_request_shapes':accepted,'rejected_mutations':negative}
(root/'schema_validation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':'passed','accepted':len(accepted),'rejected':len(negative)}))
