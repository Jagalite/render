import json
from pathlib import Path
obj=lambda props:dict(type='object',additionalProperties=False,properties=props,required=list(props))
ref=lambda n:{'$ref':'#/$defs/'+n}
integer=lambda lo,hi:dict(type='integer',minimum=lo,maximum=hi)
number=lambda lo,hi:dict(type='number',minimum=lo,maximum=hi)
array=lambda items,lo,hi:dict(type='array',items=items,minItems=lo,maxItems=hi)
ID=dict(type='string',pattern='^[0-9a-f]{32}$');HASH=dict(type='string',pattern='^sha256:[0-9a-f]{64}$');unit=number(0,1)
d={}
d['paintCoordinate']=obj(dict(x=integer(0,63),y=integer(0,63)))
d['paintTileRef']=obj(dict(coordinate=ref('paintCoordinate'),tile=HASH))
d['paintTile']={'oneOf':[
 obj(dict(kind={'const':'color'},version={'const':0},rgba_le=dict(type='string',pattern='^[0-9a-f]{16384}$'))),
 obj(dict(kind={'const':'mask'},version={'const':0},coverage_le=dict(type='string',pattern='^[0-9a-f]{4096}$')))]}
d['paintMode']={'oneOf':[obj(dict(kind={'const':'paint'},color=array(unit,4,4))),obj(dict(kind={'const':'erase'},opacity=unit)),obj(dict(kind={'const':'mask'},value=unit,opacity=unit))]}
d['paintBrush']=obj(dict(radius=number(.25,128),hardness=unit,spacing=number(.05,2),mode=ref('paintMode')))
d['paintSample']=obj(dict(pixel=array(number(-4096,4096),2,2),pressure=unit,time=obj(dict(numerator=integer(0,9223372036854775807),denominator=integer(1,18446744073709551615)))))
d['paintStrokeInput']=obj(dict(id=ID,layer=ID,brush=ref('paintBrush'),samples=array(ref('paintSample'),0,256),finish=dict(type='boolean')))
d['paintStroke']=obj(dict(id=ID,layer=ID,brush=ref('paintBrush'),samples=array(ref('paintSample'),1,256),closed=dict(type='boolean')))
d['paintLayer']=obj(dict(id=ID,name=dict(type='string',minLength=1,maxLength=128),visible=dict(type='boolean'),opacity=integer(0,65535),color=array(ref('paintTileRef'),0,256),mask=array(ref('paintTileRef'),0,256)))
d['paintAsset']=obj(dict(version={'const':0},width=integer(1,2048),height=integer(1,2048),role={'enum':['srgb_color','linear_data']},layers=array(ref('paintLayer'),1,8),strokes=array(ref('paintStroke'),0,16)))
d['paintBudget']=obj(dict(max_dabs=integer(1,4096),max_pixel_work=integer(1,16777216),max_touched_tiles=integer(1,64),max_output_bytes=integer(1,4194304)))
d['paintBakeBudget']=obj(dict(max_pixel_work=integer(1,16777216),max_encoded_bytes=integer(1,4194304)))
common=dict(version={'const':0},base_revision=HASH,idempotency_key=dict(type='string',minLength=16,maxLength=128),canvas=ID,source_asset=HASH)
d['paintRequest']=obj(dict(**common,stroke=ref('paintStrokeInput'),budget=ref('paintBudget'),max_added_bytes=integer(0,67108864)))
d['paintBakeRequest']=obj(dict(**common,budget=ref('paintBakeBudget'),max_added_bytes=integer(0,67108864)))
d['paintCommand']={'oneOf':[obj(dict(op={'const':'put_paint_tile'},tile=ref('paintTile'))),obj(dict(op={'const':'put_paint_asset'},asset=ref('paintAsset'))),obj(dict(op={'const':'set_paint_canvas'},canvas=ID,source_asset={'anyOf':[HASH,{'type':'null'}]},asset={'anyOf':[HASH,{'type':'null'}]}))]}
schema={'$schema':'https://json-schema.org/draft/2020-12/schema','title':'Bounded tiled painting v0','description':'Typed wire shape; Rust additionally validates canonical times, unique coordinates and IDs, premultiplication, references, padding, aggregate work, permissions, revisions and publication.','oneOf':[ref('paintRequest'),ref('paintBakeRequest'),ref('paintCommand')],'$defs':d}
Path('schemas/tiled_painting.schema.json').write_text(json.dumps(schema,indent=2)+'\n')
p=Path('schemas/agent_request.schema.json');agent=json.loads(p.read_text());agent['$defs'].update(d)
for method,request in [('author_paint','paintRequest'),('bake_paint','paintBakeRequest')]:
 agent['properties']['operation']['oneOf'].append(obj(dict(method={'const':method},request=ref(request))))
p.write_text(json.dumps(agent,indent=2)+'\n')
