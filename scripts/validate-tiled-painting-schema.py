"""Independent strict wire-shape validation; semantic checks remain in Rust."""
import copy,json,sys
from pathlib import Path
import jsonschema
root=Path(sys.argv[1]);schema=json.loads(Path('schemas/agent_request.schema.json').read_text());validator=jsonschema.Draft202012Validator(schema)
wire=json.loads(Path('schemas/tiled_painting.schema.json').read_text());command_validator=jsonschema.Draft202012Validator(wire)
workflow=json.loads((root/'workflow_report.json').read_text());positive=[];negative=[];commands=0
for step in workflow['operations']:
 op=step['operation']
 if op['method'] in ['author_paint','bake_paint']:
  request=dict(version=0,operation=op);validator.validate(request);positive.append(request)
 elif op['method']=='apply':
  for command in op['request']['commands']:
   if command['operation'] in ['put_paint_asset','put_paint_tile','set_paint_canvas']:command_validator.validate(command);commands+=1
for name,mutate in [
 ('unknown-request-field',lambda r:r.update(private_offset=0)),
 ('future-version',lambda r:r.update(version=1)),
 ('bad-hash',lambda r:r.update(source_asset='missing')),
 ('bad-id',lambda r:r.update(canvas=0)),
 ('excess-work',lambda r:r['budget'].update(max_pixel_work=16777217)),
 ('zero-tiles',lambda r:r['budget'].update(max_touched_tiles=0)),
 ('wrong-radius',lambda r:r['stroke']['brush'].update(radius=129)),
 ('inert-tilt',lambda r:r['stroke']['samples'][0].update(tilt=[0,0])),
 ('unknown-time-field',lambda r:r['stroke']['samples'][0]['time'].update(unit='seconds')),
 ('pressure-range',lambda r:r['stroke']['samples'][0].update(pressure=2)),
 ('negative-time',lambda r:r['stroke']['samples'][0]['time'].update(numerator=-1)),
 ('zero-denominator',lambda r:r['stroke']['samples'][0]['time'].update(denominator=0)),
 ('unsupported-mode',lambda r:r['stroke']['brush']['mode'].update(kind='multiply')),
 ('color-shape',lambda r:r['stroke']['brush']['mode'].update(color=[1,0,0])),
]:
 r=copy.deepcopy(positive[0]);mutate(r['operation']['request']);assert list(validator.iter_errors(r)),name;negative.append(name)
report=dict(status='passed',positive=len(positive),commands=commands,negative=negative,boundary='Wire shape only; Rust additionally enforces integrity, premultiplication, uniqueness, canonical time, aggregate budgets and atomic publication.')
(root/'schema_validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
