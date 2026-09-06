"""Original CC0 named-UV coverage recipes; creates a new directory, no renderer."""
import copy,hashlib,json,struct,sys
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=False)
source=Path(__file__).resolve().parents[1]/'named-uv/data'
root=json.loads((source/'roles.gltf').read_text());binary=(source/'roles.bin').read_bytes()
for name in ['named-mask','named-blend','ao-mask','boundary-mask','subnormal-mask']:
    r=copy.deepcopy(root);r['asset']['generator']='Original named UV alpha coverage fixture'
    m=r['materials'][0];m['alphaMode']='BLEND' if name=='named-blend' else 'MASK'
    m['alphaCutoff']=0.5
    m['pbrMetallicRoughness']['baseColorTexture']['texCoord']=1
    if name=='named-blend':m['pbrMetallicRoughness']['baseColorFactor'][3]=0.5
    if name in ['boundary-mask','subnormal-mask']:
        factor=.0031 if name=='boundary-mask' else float.fromhex('0x0.0000000000001p-1022')
        m['pbrMetallicRoughness']['baseColorFactor'][3]=factor
        m['alphaCutoff']=struct.unpack('<f',struct.pack('<f',85/255))[0]*factor if name=='boundary-mask' else factor
    if name=='ao-mask':
        m['pbrMetallicRoughness']={'baseColorFactor':[0.5,0.5,0.5,1],'metallicFactor':0,'roughnessFactor':0.5}
        m['emissiveFactor']=[0,0,0];m.pop('emissiveTexture');m.pop('normalTexture')
    r['buffers'][0]['uri']=name+'.bin';(out/(name+'.bin')).write_bytes(binary)
    (out/(name+'.gltf')).write_text(json.dumps(r,indent=2)+'\n')
    del r['buffers'][0]['uri'];j=json.dumps(r,separators=(',',':')).encode();j+=b' '*(-len(j)%4);b=binary+b'\0'*(-len(binary)%4)
    (out/(name+'.glb')).write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(b))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(b),0x004e4942)+b)
(out/'provenance.json').write_text(json.dumps({'license':'CC0-1.0','authorship':'Original named-UV fixture with explicit coverage/AO recipes','inputs':[{'file':'fixtures/named-uv/data/'+p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in [source/'roles.gltf',source/'roles.bin']],'oracle':'Coverage samples UV1=(y,x), independently of emission UV1 and AO UV2; decoded alpha bytes 0,85,170,255. MASK cutoff .5, BLEND factor .5. AO-only MASK has full coverage and indirect factor from red channel of texture2 at UV2.','artifacts':[{'file':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(out.iterdir())]},indent=2)+'\n')
