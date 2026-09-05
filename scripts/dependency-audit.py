"""Release evidence generator. Build/test tooling is outside the application runtime."""
import hashlib
import json
import subprocess
from pathlib import Path

def command(*args):
    return subprocess.check_output(args,text=True)

def inventory(target,root_name):
    metadata=json.loads(command('cargo','metadata','--locked','--offline','--format-version','1','--filter-platform',target))
    packages={p['id']:p for p in metadata['packages']}
    nodes={n['id']:n for n in metadata['resolve']['nodes']}
    root=next(p['id'] for p in packages.values() if p['name']==root_name)
    selected=set()
    def visit(id):
        if id in selected:return
        selected.add(id)
        for dep in nodes[id]['deps']:
            if any(kind['kind']!='dev' for kind in dep['dep_kinds']):visit(dep['pkg'])
    visit(root)
    results=[]
    for id in sorted(selected):
        p=packages[id]
        results.append({'name':p['name'],'version':p['version'],'license_expression':p['license'],'source':p['source'],'native_links':p.get('links'),'features':nodes[id]['features'],'build_script':any('custom-build' in t['kind'] for t in p['targets'])})
    forbidden={'blender','cycles','embree','embree-sys','assimp-sys','shaderc-sys','ffmpeg-sys-next','openssl-sys','zstd-sys','libz-sys','oidn','openvdb','opencv','python3-sys','pyo3','sha2-asm','cc','cmake'}
    violations=[p for p in results if p['name'] in forbidden]
    # Platform bindings are reviewed environmental exceptions, not computational fallbacks.
    linked=[p for p in results if p['native_links']]
    allowed_links={'wasm_bindgen','wasm-bindgen','core-graphics','metal','objc'}
    violations += [p for p in linked if p['native_links'] not in allowed_links]
    return {'target':target,'root':root_name,'packages':results,'violations':violations,'scope':'normal and build dependency edges, excluding dev-only test/benchmark dependencies'}

root=Path('artifacts/evidence');root.mkdir(parents=True,exist_ok=True)
native=inventory('aarch64-apple-darwin','render-host')
web=inventory('wasm32-unknown-unknown','render-web')
links=command('otool','-L','target/debug/render-host')
paths=[line.strip().split(' (')[0] for line in links.splitlines()[1:]]
unexpected=[path for path in paths if not path.startswith(('/System/Library/','/usr/lib/'))]
artifacts=[]
for path in [Path('web/pkg/render_web_bg.wasm'),Path('web/pkg/render_web.js'),Path('web/pkg/bootstrap.js')]:
    if path.exists():artifacts.append({'path':str(path),'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'generator':'scripts/build-web.sh; Rust render-web; wasm-bindgen 0.2.104'})
report={'status':'passed' if not native['violations'] and not web['violations'] and not unexpected else 'failed','native':native,'browser':web,'native_link_inspection':{'paths':paths,'unexpected_paths':unexpected},'environmental_exceptions':['macOS system services and graphics frameworks','browser WebGPU, DOM, workers, Web Locks, OPFS, JS engine','generated wasm-bindgen and minimal bootstrap interop'],'generated_artifacts':artifacts,'toolchain':command('rustc','--version').strip(),'lockfile_sha256':hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest(),'limitations':['validated targets are macOS aarch64 and wasm32 only','SPDX-style inventory, not an SPDX certification','denylist and native-link inspection supplement manual dependency review; not proof of arbitrary future dependency safety']}
(root/'dependency_inventory.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'status':report['status'],'native_packages':len(native['packages']),'browser_packages':len(web['packages']),'native_violations':native['violations'],'browser_violations':web['violations'],'unexpected_links':unexpected}))
if report['status']!='passed':raise SystemExit(1)
