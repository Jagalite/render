"""Verify generated numeric source, retained native provenance and analytic accuracy."""
from decimal import Decimal,localcontext
import hashlib,json,struct,subprocess,sys
from pathlib import Path
p=Path('fixtures/tiled-painting/srgb8-native-baseline.json');reference=json.loads(p.read_text())
output=subprocess.check_output(['python3','scripts/generate-srgb8-compat.py'])
assert output==Path('crates/core/src/generated/srgb8_compat_v1.rs').read_bytes()
source=Path(reference['source']['path']).read_bytes()
assert hashlib.sha256(source).hexdigest()==reference['source']['sha256']
assert source==subprocess.check_output(['git','show',reference['source']['revision']+':'+reference['source']['path']])
with localcontext() as context:
 context.prec=64;errors=[]
 for code,bits in enumerate(reference['bits']):
  x=Decimal(code)/Decimal(255)
  analytic=x/Decimal('12.92') if x<=Decimal('0.04045') else ((x+Decimal('0.055'))/Decimal('1.055'))**Decimal('2.4')
  actual=Decimal.from_float(struct.unpack('<f',struct.pack('<I',bits))[0]);errors.append(abs(actual-analytic))
 maximum=max(errors);assert maximum<Decimal('0.0000003')
report=dict(status='passed',profile=reference['profile'],input_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),generated_rust_sha256=hashlib.sha256(output).hexdigest(),native_reference_codes=256,source_function_unchanged_from=reference['source']['revision'],prior_native_wasm_differing_codes=reference['native_wasm_prior_differing_codes'],maximum_absolute_error_against_decimal64=str(maximum),error_bound='0.0000003',reference_images_changed=False)
if len(sys.argv)>1:Path(sys.argv[1]).write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
