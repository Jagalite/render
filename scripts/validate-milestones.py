"""Replay M06-M08 and inherited gates into a fresh evidence directory.

Requires the Rust web server :8765 and isolated Chrome CDP :9223. This script
never updates checked-in evidence or reference images. See docs/m07_m08.md.
"""
import datetime
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import time

root = Path('artifacts/m06-m08') / datetime.datetime.now(datetime.timezone.utc).strftime('run-%Y%m%dT%H%M%SZ')
logs = root / 'logs'
logs.mkdir(parents=True)
report = {'status': 'in_progress', 'root': str(root), 'checks': [], 'resources': {},
          'scope': 'macOS ARM64 native CPU/Metal; Chromium Rust Wasm/WebGPU; Linux/Windows compile only'}

def save():
    (root / 'validation_report.json').write_text(json.dumps(report, indent=2) + '\n')

def run(name, command, resources=False):
    at = time.monotonic()
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (logs / (name + '.txt')).write_text(result.stdout)
    report['checks'].append({'name': name, 'command': command, 'exit_code': result.returncode,
                             'seconds': time.monotonic() - at, 'log': 'logs/' + name + '.txt'})
    if resources and result.returncode == 0:
        report['resources'][name] = {label: int(re.search(r'(\d+)\s+' + label, result.stdout).group(1))
                                    for label in ['maximum resident set size', 'peak memory footprint']}
    if result.returncode:
        report['status'] = 'failed'
    save()
    print(f'{name}: exit {result.returncode}', flush=True)
    if result.returncode:
        print(result.stdout[-7000:], flush=True)
        raise SystemExit(result.returncode)

print(root, flush=True)
run('regressions', ['python3', 'scripts/validate-agent.py'])
# Copy reports immediately, so later runs cannot change this run's foundation record.
import shutil
shutil.copytree('artifacts/m05', root / 'foundation', ignore=shutil.ignore_patterns('native-*'))
shutil.copy2('artifacts/evidence/dependency_inventory.json', root / 'dependency_inventory.json')
for kind in ['modeling', 'geometry', 'groom', 'volume', 'surfaces', 'imaging', 'animation']:
    target = root / (('character' if kind == 'animation' else kind) + '-01')
    run(kind, ['/usr/bin/time', '-l', 'target/debug/render-host', kind + '-workflow', str(target)], True)
run('browser_geometry', ['node', 'scripts/m07-browser-conformance.mjs', str(root), 'geometry,volume,groom,modeling'])
run('browser_products', ['node', 'scripts/m07-products-browser.mjs', str(root)])
run('browser_animation', ['node', 'scripts/m08-browser-conformance.mjs', str(root / 'character-01')])
run('static_pbr_native', ['/usr/bin/time', '-l', 'target/debug/render-host', 'pbr-workflow', str(root / 'static-pbr')], True)
run('static_pbr_browser', ['node', 'scripts/static-pbr-browser-conformance.mjs'])
run('static_pbr_comparison', ['python3', 'scripts/static-pbr-compare.py', str(root / 'static-pbr')])
shutil.copy2('artifacts/static-pbr/browser_workflow_report.json', root / 'static_pbr_browser_report.json')
shutil.copy2('artifacts/static-pbr/cross_platform_report.json', root / 'static_pbr_cross_platform_report.json')

# Independently compare complete native and browser passes, including shutter frames.
def compare(native, browser):
    header, dims, endian, payload = native.with_suffix('.pfm').read_bytes().split(b'\n', 3)
    assert header == b'PF' and endian == b'-1.0'
    width, height = map(int, dims.split())
    values = struct.unpack('<' + 'f' * (width * height * 3), payload)
    rgb = [x for y in range(height - 1, -1, -1) for x in values[y * width * 3:(y + 1) * width * 3]]
    nw, nh, depth, normals, objects = json.loads(native.with_suffix('.passes.json').read_text())
    assert (nw, nh) == (width, height)
    flat = lambda a: [x for row in a for x in row]
    rmse = lambda a, b: (sum((x-y)**2 for x, y in zip(a, b, strict=True)) / len(a))**0.5
    row = {'native': str(native.relative_to(root)), 'linear_rmse': rmse(rgb, flat(browser['linear_rgb'])),
           'normal_rmse': rmse(flat(normals), flat(browser['normals_world'])),
           'depth_rmse': rmse(depth, browser['depth_meters']),
           'object_mismatches': sum(x != y for x, y in zip(objects, browser['object_ids'], strict=True))}
    assert row['linear_rmse'] < 0.025 and row['normal_rmse'] < 0.025 and row['depth_rmse'] < 0.0001 and row['object_mismatches'] <= 4, row
    return row

try:
    comparisons = []
    for kind in ['geometry', 'groom', 'volume', 'modeling']:
        browser = json.loads((root / (kind + '-browser-passes.json')).read_text())
        comparisons.append(compare(root / (kind + '-01') / ('volume-cpu' if kind == 'volume' else 'geometry-cpu'), browser))
    browser = json.loads((root / 'surfaces-01/browser_report.json').read_text())
    comparisons.append(compare(root / 'surfaces-01/surfaces-cpu', browser['output']['passes']))
    for frame in [0, 2, 4, 6, 8]:
        browser = json.loads((root / f'character-01/browser-frame-{frame}.passes.json').read_text())
        comparisons.append(compare(root / f'character-01/frame-{frame:03}', browser))
    (root / 'cross_platform_report.json').write_text(json.dumps({'status': 'passed', 'comparisons': comparisons}, indent=2) + '\n')
    # Provenance remains pinned for external texture fixtures.
    for path in sorted(Path('fixtures/static-pbr').glob('*/provenance.json')):
        for item in json.loads(path.read_text())['files']:
            data = (path.parent / item['file']).read_bytes()
            assert len(data) == item['bytes'] and hashlib.sha256(data).hexdigest() == item['sha256']
    paths = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard'], text=True).splitlines()
    manifest = []
    for name in sorted(set(paths)):
        path = Path(name)
        if path.is_file() and not name.startswith(('evidence/', 'artifacts/')):
            manifest.append({'path': name, 'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    (root / 'source_manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
except Exception as error:
    report['status'] = 'failed'
    report['comparison_or_provenance_error'] = repr(error)
    save()
    raise
report['status'] = 'passed'
report['completed_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
save()
print('M06-M08 validation passed: ' + str(root), flush=True)
