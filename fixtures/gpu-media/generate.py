"""Original analytic sparse-medium corpus; no external model or runtime."""
import copy,json,math,sys
from pathlib import Path
out=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).parent/'data';out.mkdir(parents=True,exist_ok=True)
identity={'columns':[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],'operations':[]}
def asset(sigma=(.5,1,2),j=(.2,.4,.6)):
 return {'origin':[0,0,0],'voxel_size':[1,1,1],'cells':[{'coordinate':[0,0,0],'density':1,'emission':list(j)}],'absorption':list(sigma),'scattering':[0,0,0],'anisotropy':0,'max_step_meters':.1}
def medium(a,t=None):return {'asset':a,'transform':copy.deepcopy(t or identity)}
def expected(segments):
 t=[1.]*3;r=[0.]*3
 for sigma,j,length in segments:
  for i in range(3):
   integral=-math.expm1(-sigma[i]*length)/sigma[i] if sigma[i] else length
   r[i]+=t[i]*j[i]*integral;t[i]*=math.exp(-sigma[i]*length)
 return [r[i]+t[i]*[.3,.5,.7][i] for i in range(3)]
cases=[]
def add(name,media,segments,camera=(.5,.5,4),near=.01,far=10,target=None):
 cases.append({'name':name,'media':media,'camera':{'position':list(camera),'target':list(target or (camera[0],camera[1],camera[2]-1)),'near':near,'far':far},'segments_front_to_back':[{'extinction':s,'emission':j,'length':l} for s,j,l in segments],'expected':expected(segments)})
for name,sigma in [('homogeneous',[.5,1,2]),('vacuum',[0,0,0]),('tiny',[1e-9,1e-7,1e-5]),('small-optical-depth',[.0099,.01,.0101]),('series-boundary',[.0999,.1,.1001]),('opaque',[100,1000,1e6])]:
 a=asset(sigma);add(name,[medium(a)],[(sigma,a['cells'][0]['emission'],1)])
a=asset();s=a['absorption'];j=a['cells'][0]['emission'];add('overlap',[medium(a),medium(a)],[( [2*x for x in s],[2*x for x in j],1)])
b=asset([1.5,.2,.7],[.8,.1,.3]);t=copy.deepcopy(identity);t['columns'][3][2]=.5
add('ordered-overlap',[medium(a),medium(b,t)],[(b['absorption'],b['cells'][0]['emission'],.5),([x+y for x,y in zip(s,b['absorption'])],[x+y for x,y in zip(j,b['cells'][0]['emission'])],.5),(s,j,.5)])
sparse=copy.deepcopy(a);sparse['cells'].append(dict(coordinate=[0,0,2],density=1,emission=j));add('sparse-hole',[medium(sparse)],[(s,j,1),(s,j,1)])
t=copy.deepcopy(identity);t['columns'][2][2]=2;add('metric-scale',[medium(a,t)],[(s,j,2)])
t=copy.deepcopy(identity);t['columns'][0][0]=-1;add('negative-scale',[medium(a,t)],[(s,j,1)],camera=(-.5,.5,4))
t=copy.deepcopy(identity);t['columns'][2][0]=.5;add('sheared',[medium(a,t)],[(s,j,1)],camera=(.75,.5,4))
add('near-clip',[medium(a)],[(s,j,.5)],near=3.5)
add('far-clip',[medium(a)],[(s,j,.5)],far=3.5)
# Orthographic Camera.clip retains its documented 1e-5m minimum.
add('inside',[medium(a)],[(s,j,.5-1e-5)],camera=(.5,.5,.5),near=0)
add('miss',[medium(a)],[],camera=(2,.5,4))
add('zero-density-emission',[medium(dict(a,cells=[dict(coordinate=[0,0,0],density=0,emission=j)]))],[([0,0,0],j,1)])
(out/'cases.json').write_text(json.dumps({'license':'CC0-1.0','author':'Render project original analytic corpus','generator':'fixtures/gpu-media/generate.py','environment':[.3,.5,.7],'cases':cases},indent=2)+'\n')
