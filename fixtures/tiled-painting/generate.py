"""Original CC0-1.0 painting inputs. No output images or automatic goldens."""
import json,sys
from pathlib import Path
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True)
def identity(n):return f'{n:032x}'
def layer(n,name):return dict(id=identity(n),name=name,visible=True,opacity=65535,color=[],mask=[])
def sample(x,y,t,pressure=1):return dict(pixel=[x,y],pressure=pressure,time=dict(numerator=t,denominator=1))
def stroke(n,layer,mode,samples,finish=True,radius=6,hardness=.5,spacing=.4):
 return dict(id=identity(n),layer=identity(layer),brush=dict(radius=radius,hardness=hardness,spacing=spacing,mode=mode),samples=samples,finish=finish)
canvas=dict(version=0,width=64,height=64,role='srgb_color',layers=[layer(901,'Ochre base'),layer(902,'Blue detail')],strokes=[])
mode=dict(kind='paint',color=[.8,.3,.04,1])
inputs=dict(canvas_id=identity(900),canvas=canvas,strokes=[
 stroke(910,901,mode,[sample(5,10,0),sample(30,20,1,.6)],False,radius=10),
 stroke(910,901,mode,[sample(52,45,2)],True,radius=10),
 stroke(911,902,dict(kind='paint',color=[.02,.2,.9,.8]),[sample(12,50,0),sample(48,8,1)],radius=8),
 stroke(912,902,dict(kind='mask',value=.1,opacity=.9),[sample(31,31,0),sample(39,31,1)],radius=7),
 stroke(913,901,dict(kind='erase',opacity=.6),[sample(18,18,0)],radius=3),
])
(root/'layered-strokes.json').write_text(json.dumps(inputs,indent=2)+'\n')
