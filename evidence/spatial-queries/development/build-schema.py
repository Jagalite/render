import json
from pathlib import Path
def obj(properties):return dict(type='object',additionalProperties=False,properties=properties,required=list(properties))
def integer(cap):return dict(type='integer',minimum=1,maximum=cap)
vector=dict(type='array',minItems=3,maxItems=3,items=dict(type='number',minimum=-1e9,maximum=1e9))
metric=dict(type='number',minimum=0,maximum=1e9)
query=dict(oneOf=[obj(dict(kind=dict(const='points_in_sphere'),center=vector,radius_meters=metric)),obj(dict(kind=dict(const='nearest_surface'),point=vector,max_distance_meters=metric))])
hash=dict(type='string',pattern='^sha256:[0-9a-f]{64}$')
budget=obj({key:integer(cap) for key,cap in dict(max_source_bytes=8<<20,max_build_bytes=32<<20,max_visited_nodes=262144,max_item_tests=131072,max_hits=4096,max_result_bytes=1<<20).items()})
request=obj(dict(version=dict(const=0),base_revision=hash,mesh=hash,query=query,budget=budget))
p=Path('schemas/agent_request.schema.json');schema=json.loads(p.read_text());schema['properties']['operation']['oneOf'].append(obj(dict(method=dict(const='query_geometry'),request=request)));p.write_text(json.dumps(schema,indent=2)+'\n')
wire={'$schema':'https://json-schema.org/draft/2020-12/schema','title':'Mesh-local spatial query v0','description':'Read-only asset-local meters. Rust additionally validates content identity, topology, revisions, aggregate budgets and cancellation. New element IDs in responses are positive canonical decimal u64 strings.',**request};Path('schemas/spatial_query.schema.json').write_text(json.dumps(wire,indent=2)+'\n')
