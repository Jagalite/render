"""Structural HAIR DTO validation using real import requests and negative policy cases."""
import copy,hashlib,json,sys
from pathlib import Path
from jsonschema import Draft202012Validator
root=Path(sys.argv[1]);schema=Path('schemas/agent_request.schema.json');definition=json.loads(schema.read_text());Draft202012Validator.check_schema(definition);validator=Draft202012Validator(definition);accepted=[]
workflow=json.loads((root/'workflow_report.json').read_text());assert workflow['status']=='passed'
for row in workflow['variants']:
 q={'version':0,'operation':{'method':'import_hair','bytes':list(Path(row['source']['path']).read_bytes()),'policy':row['source']['policy'],'settings':row['settings'],'base_revision':row['empty_revision'],'idempotency_key':'hair:import:0001'}};validator.validate(q);accepted.append(row['name']);base=q
negative=[]
for field,value in [('byte_order','auto'),('meters_per_unit',0),('thickness','auto'),('color_space','auto'),('transparency','ignore'),('representation','fiber_bsdf'),('roughness',2),('unknown',True)]:
 q=copy.deepcopy(base);q['operation']['policy'][field]=value;assert list(validator.iter_errors(q));negative.append(field)
for field in base['operation']['policy']:
 q=copy.deepcopy(base);del q['operation']['policy'][field];assert list(validator.iter_errors(q));negative.append('missing '+field)
for field,value in [('max_vertices',65537),('max_samples',32769),('max_depth',0),('chord_error',0),('radial_error',0)]:
 q=copy.deepcopy(base);q['operation']['policy']['tessellation'][field]=value;assert list(validator.iter_errors(q));negative.append('tessellation '+field)
q=copy.deepcopy(base);q['operation']['bytes']=[-1];assert list(validator.iter_errors(q));negative.append('invalid byte')
(root/'schema_validation.json').write_text(json.dumps({'status':'passed','accepted':accepted,'rejected':negative,'schema_sha256':hashlib.sha256(schema.read_bytes()).hexdigest(),'scope':'Structural DTO subset; binary counts, finite data, references, revisions and derived geometry costs require shared Rust checks.'},indent=2)+'\n');print('Passed:',len(accepted),'positive,',len(negative),'negative')
