"""Derive acceptance indexes from an already passed M06-M08 validation run.

Writes only into the supplied artifacts directory; never installs golden images.
"""
import hashlib
import json
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
read = lambda name: json.loads((root / name).read_text())
validation = read('validation_report.json')
assert validation['status'] == 'passed'
foundation = read('foundation/validation_report.json')
assert foundation['status'] == 'passed' and len(foundation['checks']) == 17
native_log = (root / 'foundation/logs/native_tests.txt').read_text()
wasm_log = (root / 'foundation/logs/wasm_tests.txt').read_text()
counts = {kind: sum(map(int, re.findall(r'test result: ok\. (\d+) passed', log)))
          for kind, log in [('native', native_log), ('wasm', wasm_log)]}

def save(name, value):
    (root / name).write_text(json.dumps(value, indent=2) + '\n')

def cases(files):
    result = []
    for file in files:
        source = Path('crates/core/tests/' + file + '.rs')
        # Test functions are the functions immediately following paired native/Wasm attributes.
        names = re.findall(r'#\[cfg_attr\(not\(target_arch = "wasm32"\), test\)\]\s*fn (\w+)\(', source.read_text())
        assert names, file
        for name in names:
            assert re.search(r'\b' + re.escape(name) + r'\s+\.\.\.\s+ok', native_log), name
            assert name in wasm_log and 'FAILED' not in wasm_log, name
            result.append({'case': name, 'source': str(source), 'native': 'passed', 'wasm': 'passed'})
    return result

save('modeling_conformance.json', {'status': 'passed', 'tests': cases(['modeling']),
     'native_workflow': 'modeling-01/report.json', 'procedural': 'modeling-01/procedural_conformance.json',
     'topology_and_edit_amplification': 'modeling-01/topology_mapping_report.json'})
save('procedural_conformance.json', read('modeling-01/procedural_conformance.json'))
save('topology_mapping_report.json', read('modeling-01/topology_mapping_report.json'))
save('geometry_family_report.json', {'status': 'passed', 'tests': cases(['curves', 'groom', 'volumes']),
     'workflows': {name: read(name + '-01/report.json') for name in ['geometry', 'groom', 'volume']},
     'browser': read('browser_report.json'), 'comparisons': 'cross_platform_report.json'})
save('render_feature_conformance.json', {'status': 'passed', 'tests': cases(['scattering', 'displacement', 'products']),
     'workflows': {name: read(name + '-01/report.json') for name in ['surfaces', 'imaging']},
     'browser': ['surfaces-01/browser_report.json', 'imaging-01/browser_report.json'],
     'inherited_pbr': 'static_pbr_cross_platform_report.json'})
xyz = [506752/1228815, 87098/409605, 7918/409605]
rows = [[446124/178915, -333277/357830, -72051/178915],
        [-14852/17905, 63121/35810, 423/17905], [11844/330415, -50337/660830, 316169/330415]]
save('color_profile_vectors.json', {'status': 'passed', 'source': 'https://www.w3.org/TR/css-color-4/',
     'method': 'Expected vectors from independently tabulated rational XYZ matrices; production derives matrices from chromaticities. Tests assert actual Rust output on native and Wasm.',
     'vectors': [{'input': [1,0,0], 'from': 'linear_srgb', 'to': 'linear_display_p3',
                  'expected': [sum(a*b for a,b in zip(row,xyz)) for row in rows], 'absolute_tolerance': 1e-12},
                 {'input': [0.0031308]*3, 'from': 'linear_srgb', 'to': 'srgb', 'expected': [0.040449936]*3, 'absolute_tolerance': 1e-9},
                 {'input': [1]*3, 'from': 'linear_srgb', 'to': 'all_four_profiles', 'expected': [1]*3, 'absolute_tolerance': 1e-12}],
     'tests': cases(['products'])})
animation = cases(['animation'])
save('animation_conformance.json', {'status': 'passed', 'tests': animation,
     'native': read('character-01/report.json'), 'browser': read('character-01/browser_report.json'),
     'comparison': 'cross_platform_report.json'})
save('rig_solver_report.json', {'status': 'passed',
     'tests': [c for c in animation if any(k in c['case'] for k in ['bind_', 'skinning_', 'skin_morph'])],
     'profiles': ['copy_position', 'two_bone_ik'],
     'diagnostics': ['causal dependency cycle', 'conflicting constraint writes', 'unreachable goal', 'excessive residual', 'invalid inverse bind', 'stale topology'],
     'workflow': 'character-01/report.json', 'authored_fixture': 'character-01/document.json'})
save('motion_blur_report.json', {'status': 'passed', 'tests': [c for c in animation if 'shutter_' in c['case']],
     'request': read('character-01/request.json'), 'comparisons': read('cross_platform_report.json'),
     'policy': 'independent exact-time rigid/morph/LBS midpoint samples; nominal-time auxiliary passes; streaming completed frames; partial output on cancellation'})
prior = {x['path']: x for x in json.loads(Path('evidence/static-pbr/source_manifest.json').read_text())}
lock = hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest()
assert lock == prior['Cargo.lock']['sha256'], 'dependency delta needs new manual provenance review'
save('dependency_provenance.json', {'status': 'passed', 'lockfile_sha256': lock, 'changed_since_static_pbr': False,
     'inventory': 'dependency_inventory.json', 'synthetic_fixture_generator': 'crates/core/src/feature_fixtures.rs',
     'fixture_authorship': 'Original synthetic geometry, materials, rig and animation authored in this repository; no third-party model source.',
     'external_fixtures': 'Existing fixtures/static-pbr/*/provenance.json revalidated by the runner.',
     'generated_inputs': ['crates/kernel/src/path.rs', 'crates/kernel/src/path/', 'crates/gpu/src/lib.rs', 'crates/web/', 'scripts/build-web.sh'],
     'runtime_exceptions': read('dependency_inventory.json')['environmental_exceptions']})
save('resource_measurements.json', {'status': 'passed', 'native_process_bytes': validation['resources'],
     'native_workflows': {name: {key: value for key, value in read(name + '-01/report.json').items()
                               if key in ['seconds', 'evaluation_seconds', 'source_snapshot_bytes', 'conversions', 'displacements', 'procedures', 'occupied_cells']}
                          for name in ['modeling','geometry','groom','volume','surfaces','imaging','character']},
     'modeling_local_wide_edits': 'topology_mapping_report.json', 'browser': ['browser_report.json','imaging-01/browser_report.json','surfaces-01/browser_report.json','character-01/browser_report.json'],
     'limits': 'docs/m06_modeling_contract.md and docs/m07_m08.md',
     'qualification': 'Single macOS ARM64 development machine; debug native and release Wasm. Process RSS/footprint and serialized growth are distinct metrics; not a benchmark across platforms.'})
save('acceptance_index.json', {'status': 'passed', 'test_counts': counts, 'foundation_gates': 17,
     'milestones': ['M06', 'M07', 'M08'], 'scope': 'Complete canonical milestone gates for the documented named experimental profiles; advanced compatibility extensions remain separate.',
     'evidence': ['modeling_conformance.json','procedural_conformance.json','topology_mapping_report.json','geometry_family_report.json','render_feature_conformance.json','color_profile_vectors.json','animation_conformance.json','rig_solver_report.json','motion_blur_report.json','resource_measurements.json','dependency_provenance.json'],
     'validation': 'validation_report.json'})
# Independent comparison of browser and native imaging products.
import math
imaging = root / 'imaging-01'
browser = read('imaging-01/browser_report.json')['output']
comparisons = []
def rgb_rmse(a, b):
    return math.sqrt(sum((x-y)**2 for r, s in zip(a, b, strict=True)
                         for x, y in zip(r, s, strict=True)) / (len(a)*3))
for view in browser['views']:
    native = json.loads((imaging / (view['name'] + '.display.json')).read_text())
    albedo = json.loads((imaging / (view['name'] + '.albedo.json')).read_text())
    row = {'view': view['name'], 'display_rmse': rgb_rmse(native['rgb'], view['display']['rgb']),
           'albedo_rmse': rgb_rmse(albedo, view['albedo'])}
    assert row['display_rmse'] < 1e-6 and row['albedo_rmse'] < 1e-6, row
    comparisons.append(row)
for name, bake in browser['bakes'].items():
    native = json.loads((imaging / (name + '.json')).read_text())
    error = rgb_rmse(native['rgb'], bake['rgb'])
    assert error < 1e-6 and native['covered'] == bake['covered'], name
    comparisons.append({'bake': name, 'rmse': error, 'coverage_equal': True})
save('imaging_cross_platform_report.json', {'status': 'passed', 'comparisons': comparisons,
     'method': 'Independent Python comparison of stored native and BrowserAgent Rust outputs; RGB RMSE < 1e-6 and identical coverage.',
     'command': 'python3 scripts/summarize-milestones.py ' + str(root)})
print(json.dumps({'status': 'passed', 'test_counts': counts, 'root': str(root)}))
