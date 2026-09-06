"""Structural schema checks over actual VOL requests; binary and semantic checks stay Rust."""
import copy,hashlib,json,sys
from pathlib import Path
from jsonschema import Draft202012Validator
root=Path(sys.argv[1]);schema=Path('schemas/agent_request.schema.json');definition=json.loads(schema.read_text());Draft202012Validator.check_schema(definition);validator=Draft202012Validator(definition)
accepted=[]
for row in json.loads((root/'workflow_report.json').read_text())['variants']:
 source=row['source'];q={'version':0,'operation':{'method':'import_vol','bytes':list(Path(source['path']).read_bytes()),'emission_bytes':list(Path(source['emission']).read_bytes()) if source['emission'] else None,'policy':source['policy'],'settings':row['settings'],'base_revision':row['empty_revision'],'idempotency_key':'volume:import:0001'}}
 validator.validate(q);accepted.append(row['name'])
 if row['name']=='homogeneous':base=q
negative=[]
for field,value in [('bounds','clamped'),('reconstruction','trilinear'),('meters_per_unit',0),('density_scale',-1),('emission_scale',[1,2]),('anisotropy',1),('max_step_meters',0),('unknown',True)]:
 q=copy.deepcopy(base);q['operation']['policy'][field]=value;assert list(validator.iter_errors(q));negative.append(field)
for field in base['operation']['policy']:
 q=copy.deepcopy(base);del q['operation']['policy'][field];assert list(validator.iter_errors(q));negative.append('missing '+field)
q=copy.deepcopy(base);q['operation']['bytes']=[256];assert list(validator.iter_errors(q));negative.append('invalid byte')
(root/'schema_validation.json').write_text(json.dumps({'status':'passed','accepted':accepted,'rejected':negative,'schema_sha256':hashlib.sha256(schema.read_bytes()).hexdigest(),'scope':'Structural DTO subset; cross-field, binary, aggregate bytes, revisions, permissions and native asset bounds require Rust semantic checks.'},indent=2)+'\n')
print('Schema passed:',len(accepted),'positive,',len(negative),'negative')
