// Development-only independent Khronos validator; never part of the product runtime.
const fs = require('node:fs');
const cp = require('node:child_process');
const path = require('node:path');
const crypto = require('node:crypto');
const validatorPath = process.env.GLTF_VALIDATOR_MODULE || '/private/tmp/render-gltf-validator-20260906/node_modules/gltf-validator';
const validator = require(validatorPath);
(async () => {
  const output = path.resolve(process.argv[2]);
  fs.mkdirSync(output, {recursive: false});
  const files = cp.execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', 'fixtures'], {encoding: 'utf8'}).trim().split('\n').filter(p => p.endsWith('.glb'));
  const results = [];
  for (const file of files) {
    const bytes = fs.readFileSync(file);
    const report = await validator.validateBytes(new Uint8Array(bytes), {
      uri: file, maxIssues: 1000,
      externalResourceFunction: () => Promise.reject(new Error('This corpus check admits embedded GLB resources only')),
    });
    const reportFile = file.replaceAll('/', '__') + '.json';
    fs.writeFileSync(path.join(output, reportFile), JSON.stringify(report, null, 2) + '\n');
    results.push({file, sha256: crypto.createHash('sha256').update(bytes).digest('hex'), errors: report.issues.numErrors, warnings: report.issues.numWarnings, infos: report.issues.numInfos, report: reportFile});
  }
  const status = results.every(r => r.errors === 0) ? 'passed' : 'failed';
  const packageJson = JSON.parse(fs.readFileSync(path.join(validatorPath, 'package.json')));
  const toolFiles = ['index.js', 'gltf_validator.dart.js', 'package.json'].filter(p => fs.existsSync(path.join(validatorPath, p))).map(p => ({file: p, sha256: crypto.createHash('sha256').update(fs.readFileSync(path.join(validatorPath,p))).digest('hex')}));
  fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify({status, validator: validator.version(), package: packageJson, development_only: true, tool_files: toolFiles, results}, null, 2)+'\n');
  console.log(JSON.stringify({status, assets: results.length, errors: results.reduce((n,r)=>n+r.errors,0), warnings: results.reduce((n,r)=>n+r.warnings,0)}));
  if (status !== 'passed') process.exitCode=1;
})().catch(e => {console.error(e); process.exitCode=1;});
