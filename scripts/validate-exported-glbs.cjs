// Independent development-only Khronos validation of the actual CLI export files.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const modulePath=process.env.GLTF_VALIDATOR_MODULE||'/private/tmp/render-gltf-validator-20260906/node_modules/gltf-validator';const validator=require(modulePath);
(async()=>{
 const root=path.resolve(process.argv[2]);const output=path.join(root,'export-validation');fs.mkdirSync(output);
 const workflow=JSON.parse(fs.readFileSync(path.join(root,'workflow_report.json')));const results=[];
 for(const row of workflow.variants){const file=path.join(root,row.name+'-export/scene/scene.glb');const bytes=fs.readFileSync(file);const report=await validator.validateBytes(new Uint8Array(bytes),{uri:row.name+'.glb',maxIssues:1000,externalResourceFunction:()=>Promise.reject(new Error('Export must embed every resource'))});fs.writeFileSync(path.join(output,row.name+'.json'),JSON.stringify(report,null,2)+'\n');results.push({name:row.name,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),errors:report.issues.numErrors,warnings:report.issues.numWarnings,infos:report.issues.numInfos});}
 const summary={status:results.every(r=>r.errors===0)?'passed':'failed',validator:validator.version(),development_only:true,results};fs.writeFileSync(path.join(output,'summary.json'),JSON.stringify(summary,null,2)+'\n');console.log(JSON.stringify(summary));if(summary.status!=='passed')process.exitCode=1;
})().catch(e=>{console.error(e);process.exitCode=1;});
