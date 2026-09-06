from pathlib import Path
import hashlib,json,re,shutil,struct
root=Path('artifacts/alpha-gltf/run-20260906T054824Z');logs=root/'logs'
report=json.loads((root/'validation_report.json').read_text())
browser=json.loads((root/'workflow/browser_report.json').read_text());assert browser['status']=='passed'
assert all(r['stale_branch_rejected'] and r['stale_render_rejected'] and r['nonempty_import_error'].startswith('import_target:') for r in browser['results'])
report['followup_checks']=[{'name':'browser_workflow_final','command':['node','scripts/alpha-gltf-browser.mjs',str(root/'workflow')],'exit_code':0,'log':'logs/browser_workflow_final.txt','script_sha256':hashlib.sha256(Path('scripts/alpha-gltf-browser.mjs').read_bytes()).hexdigest()}]
report['resolved_failures']={'browser_workflow':'browser_workflow_final'}
report['development_note']='Initial browser client expected stale_revision from a second import; existing BrowserAgent requires an empty import target and returns import_target first. Final client checks that admission error plus separate stale render and branch mutation rejection. Rust code and tested Wasm were unchanged.'
def pfm(path):
 h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0';v=struct.unpack('<'+'f'*(w*y*3),b)
 return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
comparisons=[]
for variant in ['mask','blend','opaque']:
 native=pfm(root/f'workflow/{variant}-cpu/image/image.pfm');passes=json.loads((root/f'workflow/{variant}-cpu/image/passes.json').read_text())
 b=json.loads((root/f'workflow/browser-{variant}-cpu.json').read_text());rgb=[v for p in b['linear_rgb'] for v in p]
 rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
 assert rmse==0 and passes[4]==b['object_ids'],(variant,rmse)
 assert passes[2]==b['depth_meters'] and passes[3]==b['normals_world']
 comparisons.append({'variant':variant,'browser':'cpu','linear_rmse':rmse,'object_mismatches':0,'depth_and_normals_exact':True})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
for kind in ['native','wasm']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
assert all(c['exit_code']==0 or c['name'] in report['resolved_failures'] for c in report['checks'])
report['status']='passed';(root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
print(report['native_test_count'],report['wasm_test_count']);print('All native/browser alpha pixels and passes match exactly.')
