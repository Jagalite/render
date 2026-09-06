"""GPU ideal dielectric acceptance; local Chrome/server at 9224/8774."""
import datetime, hashlib, json, os, re, shutil, struct, subprocess, time
from pathlib import Path
root=Path('artifacts/gpu-dielectric')/datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ')
logs=root/'logs';logs.mkdir(parents=True)
report={'status':'in_progress','scope':'macOS ARM64 CPU/Metal, Rust Wasm in Node, Chrome WebGPU/OPFS; Linux and Windows compile only','checks':[]}
def save(): (root/'validation_report.json').write_text(json.dumps(report,indent=2)+'\n')
env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',RENDER_TEST_ORIGIN='http://127.0.0.1:8774',CARGO_NET_OFFLINE='true',CARGO_BUILD_JOBS='2',CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner')
report['build_environment']={k:env[k] for k in ['CARGO_NET_OFFLINE','CARGO_BUILD_JOBS','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER','RENDER_TEST_ORIGIN']}
checks=[('format',['cargo','fmt','--all','--check']),('independent_source_validation',['node','scripts/validate-gltf-fixtures.cjs',str(root/'source-validation')]),('gpu_dielectric',['cargo','test','-p','render-host','--test','gpu_dielectric','--locked','--offline','--','--ignored','--nocapture']),('gpu_surfaces',['cargo','test','-p','render-host','--test','gpu_surfaces','--locked','--offline','--','--ignored','--nocapture']),('secondary_gpu',['cargo','test','-p','render-host','--test','secondary_textures_gpu','--locked','--offline','--','--ignored','--nocapture']),('gpu_alpha',['cargo','test','-p','render-host','--test','gpu_alpha','--locked','--offline','--','--ignored','--nocapture']),('gltf_export_gpu',['cargo','test','-p','render-host','--test','gltf_export_gpu','--locked','--offline','--','--ignored','--nocapture']),('vertex_colors_gpu',['cargo','test','-p','render-host','--test','vertex_colors_gpu','--locked','--offline','--','--ignored','--nocapture']),('named_uv_gpu',['cargo','test','-p','render-host','--test','named_uv_gpu','--locked','--offline','--','--ignored','--nocapture']),('morph_frame_regressions',['cargo','test','-p','render-core','--test','morph_frames','--locked','--offline']),('morph_frame_gpu',['cargo','test','-p','render-host','--test','morph_frames_gpu','--locked','--offline','--','--ignored','--nocapture']),('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),('native_tests',['cargo','test','--workspace','--locked','--offline']),('wasm_tests',['cargo','test','-p','render-core','--target','wasm32-unknown-unknown','--locked','--offline']),('linux_check',['cargo','check','-p','render-host','--target','x86_64-unknown-linux-gnu','--locked','--offline']),('windows_check',['cargo','check','-p','render-host','--target','x86_64-pc-windows-msvc','--locked','--offline']),('browser_clippy',['cargo','clippy','-p','render-web','--target','wasm32-unknown-unknown','--locked','--offline','--','-D','warnings']),('native_build',['cargo','build','-p','render-host','--locked','--offline']),('browser_build',['sh','scripts/build-web.sh']),('camera_regression',['python3','scripts/alpha-camera-workflow.py',str(root/'camera-regression')]),('dependency_audit',['python3','scripts/dependency-audit.py']),('animated_regression',['python3','scripts/animated-gltf-workflow.py',str(root/'animated-regression'),'--gpu']),('shutter_regression',['python3','scripts/gpu-shutter-workflow.py',str(root/'shutter-regression')]),('sparse_regression',['python3','scripts/sparse-gltf-workflow.py',str(root/'sparse-regression')]),('alpha_workflow',['python3','scripts/alpha-gltf-workflow.py',str(root/'alpha-regression')]),('alpha_browser',['node','scripts/alpha-gltf-browser.mjs',str(root/'alpha-regression')]),('alpha_shutter_workflow',['python3','scripts/gpu-alpha-shutter-workflow.py',str(root/'alpha-shutter')]),('alpha_shutter_browser',['node','scripts/gpu-alpha-shutter-browser.mjs',str(root/'alpha-shutter')]),('named_uv_regression',['python3','scripts/named-uv-workflow.py',str(root/'named-uv-regression')]),('named_uv_browser',['node','scripts/named-uv-browser.mjs',str(root/'named-uv-regression')]),('named_uv_cross_platform',['python3','scripts/compare-named-uv-browser.py',str(root/'named-uv-regression'),str(root/'named_uv_cross_platform.json')]),('morph_workflow',['python3','scripts/morph-frames-workflow.py',str(root/'morph-regression')]),('morph_browser',['node','scripts/morph-frames-browser.mjs',str(root/'morph-regression')]),('vertex_color_workflow',['python3','scripts/vertex-colors-workflow.py',str(root/'vertex-color-regression')]),('vertex_color_browser',['node','scripts/vertex-colors-browser.mjs',str(root/'vertex-color-regression')]),('export_workflow',['python3','scripts/gltf-export-workflow.py',str(root/'workflow')]),('export_schema',['python3','scripts/validate-gltf-export-schema.py',str(root)]),('export_source_validation',['node','scripts/validate-exported-glbs.cjs',str(root/'workflow')]),('export_browser',['node','scripts/gltf-export-browser.mjs',str(root/'workflow')]),('secondary_workflow',['python3','scripts/secondary-textures-workflow.py',str(root/'secondary-workflow')]),('secondary_browser',['node','scripts/secondary-textures-browser.mjs',str(root/'secondary-workflow')])]
checks.extend([
 ('surface_workflow',['python3','scripts/gpu-surfaces-workflow.py',str(root/'surface-workflow')]),
 ('surface_browser',['node','scripts/gpu-surfaces-browser.mjs',str(root/'surface-workflow')]),
])
for name, model in [('conductor',{'kind':'conductor','eta':[.2,.9,1.1],'k':[3,2,1.5]}),('coated',{'kind':'coated','weight':.7,'ior':1.5,'roughness':.25})]:
    checks.extend([(name+'_shutter',['python3','scripts/gpu-alpha-shutter-workflow.py',str(root/(name+'-shutter')),json.dumps(model)]),(name+'_shutter_browser',['node','scripts/gpu-alpha-shutter-browser.mjs',str(root/(name+'-shutter'))])])
checks.extend([
 ('dielectric_workflow',['python3','scripts/gpu-dielectric-workflow.py',str(root/'dielectric-workflow')]),
 ('dielectric_browser',['node','scripts/gpu-dielectric-browser.mjs',str(root/'dielectric-workflow')]),
 ('dielectric_shutter',['python3','scripts/gpu-alpha-shutter-workflow.py',str(root/'dielectric-shutter'),json.dumps({'kind':'dielectric','ior':1.5})]),
 ('dielectric_shutter_browser',['node','scripts/gpu-alpha-shutter-browser.mjs',str(root/'dielectric-shutter')]),
])
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()
manifest=[{'path':n,'bytes':Path(n).stat().st_size,'sha256':hashlib.sha256(Path(n).read_bytes()).hexdigest()} for n in sorted(set(paths)) if Path(n).is_file() and not n.startswith(('artifacts/','evidence/'))]
(root/'source_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(root,flush=True)
for name,cmd in checks:
    at=time.monotonic();path=logs/(name+'.txt')
    with path.open('w') as log: r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
    report['checks'].append({'name':name,'command':cmd,'exit_code':r.returncode,'seconds':time.monotonic()-at,'log':'logs/'+name+'.txt'});save();print(name+': '+str(r.returncode),flush=True)
    if r.returncode: report['status']='failed';save();print(path.read_text()[-6000:]);raise SystemExit(r.returncode)
def pfm(path):
    h,d,e,b=path.read_bytes().split(b'\n',3);w,y=map(int,d.split());assert h==b'PF' and e==b'-1.0'
    v=struct.unpack('<'+'f'*(w*y*3),b);return [x for row in range(y-1,-1,-1) for x in v[row*w*3:(row+1)*w*3]]
comparisons=[]
for variant in [c['name'] for c in json.loads(Path('fixtures/gltf-export/cases.json').read_text())['cases']]:
    native=pfm(root/f'workflow/{variant}-round-cpu/image/image.pfm');passes=json.loads((root/f'workflow/{variant}-round-cpu/image/passes.json').read_text())
    for backend in ['cpu','gpu']:
        browser=json.loads((root/f'workflow/browser-{variant}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p]
        rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
        assert rmse<.002 and passes[4]==browser['object_ids'],(variant,backend,rmse)
        assert backend!='cpu' or native==rgb,(variant,'CPU pixels differ')
        assert len(rgb)==len(native)
        assert max(abs(a-b) for a,b in zip(passes[2],browser['depth_meters'],strict=True))<2e-5
        assert max(abs(a-b) for p,q in zip(passes[3],browser['normals_world'],strict=True) for a,b in zip(p,q,strict=True))<.002
        comparisons.append({'variant':variant,'browser':backend,'linear_rmse':rmse,'object_mismatches':0})
(root/'cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':comparisons},indent=2)+'\n')
secondary=[]
workflow=json.loads((root/'secondary-workflow/workflow_report.json').read_text())
for row in workflow['variants']:
    for filtering in ['nearest','mip']:
        name=row['name']+'-'+filtering
        native=pfm(root/f'secondary-workflow/{name}-cpu/image/image.pfm')
        passes=json.loads((root/f'secondary-workflow/{name}-cpu/image/passes.json').read_text())
        for backend in ['cpu','gpu'] if row['gpu'] else ['cpu']:
            browser=json.loads((root/f'secondary-workflow/browser-{name}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p]
            rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
            assert rmse<.002 and passes[4]==browser['object_ids'],(name,backend,rmse)
            assert backend!='cpu' or native==rgb,(name,'CPU pixel mismatch')
            assert passes[2]==browser['depth_meters'] and passes[3]==browser['normals_world']
            secondary.append({'case':name,'browser':backend,'linear_rmse':rmse,'exact_cpu_pixels':backend=='cpu','object_mismatches':0})
(root/'secondary_cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':secondary},indent=2)+'\n')
surfaces=[]
workflow=json.loads((root/'surface-workflow/workflow_report.json').read_text())
for row in workflow['variants']:
    name=row['name'];folder=row['folder']
    native=pfm(root/f'surface-workflow/{name}-cpu/{folder}/image.pfm')
    passes=json.loads((root/f'surface-workflow/{name}-cpu/{folder}/passes.json').read_text())
    for backend in ['cpu','gpu']:
        browser=json.loads((root/f'surface-workflow/browser-{name}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p]
        rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
        assert rmse<.002 and passes[4]==browser['object_ids'],(name,backend,rmse)
        assert backend!='cpu' or native==rgb,(name,'CPU pixel mismatch')
        assert max(abs(a-b) for a,b in zip(passes[2],browser['depth_meters'],strict=True))<2e-5
        assert max(abs(a-b) for p,q in zip(passes[3],browser['normals_world'],strict=True) for a,b in zip(p,q,strict=True))<.002
        surfaces.append({'case':name,'browser':backend,'linear_rmse':rmse,'exact_cpu_pixels':backend=='cpu','object_mismatches':0})
(root/'surface_cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':surfaces},indent=2)+'\n')
surfaces=[]
workflow=json.loads((root/'dielectric-workflow/workflow_report.json').read_text())
for row in workflow['variants']:
    name=row['name'];folder=row['folder']
    native=pfm(root/f'dielectric-workflow/{name}-cpu/{folder}/image.pfm')
    passes=json.loads((root/f'dielectric-workflow/{name}-cpu/{folder}/passes.json').read_text())
    for backend in ['cpu','gpu']:
        browser=json.loads((root/f'dielectric-workflow/browser-{name}-{backend}.json').read_text());rgb=[v for p in browser['linear_rgb'] for v in p]
        rmse=(sum((a-b)**2 for a,b in zip(native,rgb,strict=True))/len(native))**.5
        assert rmse<.002 and passes[4]==browser['object_ids'],(name,backend,rmse)
        assert backend!='cpu' or native==rgb,(name,'CPU pixel mismatch')
        assert max(abs(a-b) for a,b in zip(passes[2],browser['depth_meters'],strict=True))<2e-5
        assert max(abs(a-b) for p,q in zip(passes[3],browser['normals_world'],strict=True) for a,b in zip(p,q,strict=True))<.002
        surfaces.append({'case':name,'browser':backend,'linear_rmse':rmse,'exact_cpu_pixels':backend=='cpu','object_mismatches':0})
(root/'dielectric_cross_platform.json').write_text(json.dumps({'status':'passed','comparisons':surfaces},indent=2)+'\n')
for kind in ['native','wasm']:report[kind+'_test_count']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',(logs/(kind+'_tests.txt')).read_text())))
shutil.copy2('artifacts/evidence/dependency_inventory.json',root/'dependency_inventory.json')
report['status']='passed';save();print('Passed: '+str(root),flush=True)
