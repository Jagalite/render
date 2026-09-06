"""Independent development-only public JSON schema client."""
import json,sys
from pathlib import Path
from jsonschema import Draft202012Validator
root=Path(sys.argv[1]);schema=json.loads(Path('schemas/agent_request.schema.json').read_text());validator=Draft202012Validator(schema);Draft202012Validator.check_schema(schema)
report={'status':'passed','valid':0,'invalid':0}
for row in json.loads((root/'workflow_report.json').read_text())['variants']:
 op={'method':'import_blend','bytes':list(Path(row['source']['path']).read_bytes()),'policy':row['source']['policy'],'settings':row['settings'],'base_revision':row['empty_revision'],'idempotency_key':'blend:import:0001'}
 q={'version':0,'operation':op};validator.validate(q);report['valid']+=1
 for target,key,value in [(op['policy'],'execute_script',True),(op,'url','https://invalid.example/'),(op['policy'],'meters_per_unit',0)]:
  old=target.get(key);exists=key in target;target[key]=value;assert list(validator.iter_errors(q)),key;report['invalid']+=1
  if exists:target[key]=old
  else:del target[key]
 q={'version':0,'operation':{'method':'export_source','request':{'revision':row['revision'],'asset':row['report']['source_asset']}}};validator.validate(q);report['valid']+=1
 q['operation']['request']['asset']='not-a-digest';assert list(validator.iter_errors(q));report['invalid']+=1
(root/'schema_validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
