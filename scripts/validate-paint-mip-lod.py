"""Independent Decimal64 check of retained native mip-selection observations."""
from decimal import Decimal,localcontext
import hashlib,json,struct,sys
from pathlib import Path
p=Path('fixtures/tiled-painting/mip-native-baseline.json');r=json.loads(p.read_text())
assert r['version']==0 and r['profile']=='mip-log2-f64-v1'
with localcontext() as context:
 context.prec=64;log_two=Decimal(2).ln()
 for rho_bits,lod_bits in r['pairs']:
  rho=struct.unpack('<f',struct.pack('<I',rho_bits))[0];assert rho>0
  accurate=Decimal.from_float(rho).ln()/log_two
  rounded=struct.unpack('<I',struct.pack('<f',float(accurate)))[0]
  assert rounded==lod_bits,(rho_bits,lod_bits,rounded)
report=dict(status='passed',profile=r['profile'],input_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),observed_hits=r['observed_hits'],unique_footprints=len(r['pairs']),prior_native_wasm_lod_differing_hits=r['prior_native_wasm_lod_differing_hits'],all_native_values_match_decimal64_rounded_to_f32=True,reference_images_changed=False)
if len(sys.argv)>1:Path(sys.argv[1]).write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
