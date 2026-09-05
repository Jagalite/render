// Independent browser client and fault injector; all product operations are Rust.
import fs from 'node:fs/promises';
const targets = await (await fetch('http://127.0.0.1:9223/json/list')).json();
const page = targets.find(x => x.type === 'page');
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve,reject) => {socket.onopen=resolve;socket.onerror=reject;});
let sequence=0;const pending=new Map();
socket.onmessage = event => {const m=JSON.parse(event.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);if(m.error)p.reject(m.error);else p.resolve(m.result);}};
function command(method,params={}){const id=++sequence;return new Promise((resolve,reject)=>{pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await command('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
try {
await command('Page.enable');await command('Runtime.enable');await command('Network.enable');await command('Network.setCacheDisabled',{cacheDisabled:true});
await command('Page.navigate',{url:'http://127.0.0.1:8765/'});
await evaluate(`new Promise((resolve,reject)=>{const deadline=Date.now()+120000;const poll=()=>{const e=document.getElementById('report');if(e?.dataset.status==='passed')resolve(true);else if(e?.dataset.status==='failed')reject(new Error(e.textContent));else if(Date.now()>deadline)reject(new Error('page timeout'));else setTimeout(poll,100);};poll();})`);
const asset = [...await fs.readFile('fixtures/khronos-box/Box.glb')];
const report = await evaluate(`(async()=>{
 const started=performance.now(); const m=await import('/pkg/render_web.js');
 const check=(x,s)=>{if(!x)throw new Error(s);};
 const d=new m.BrowserAgent('00000000000000000000000000000700');
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const settings={width:32,height:32,samples:4,seed:42,max_depth:1,max_bytes:8388608,environment:[0.15,0.15,0.15],light:{position:[2,3,4],intensity:[20,20,20]},camera:{position:[2,1.5,3],target:[0,0,0],up:[0,1,0],vertical_fov_radians:0.6}};
 const imported=call('import_glb',{bytes:${JSON.stringify(asset)},policy:{allow_lambertian:true},settings,base_revision:call('inspect').revision,idempotency_key:'browser:import:0001'});
 const base=call('inspect');const material=Object.keys(base.snapshot.materials)[0];
 const name='m05-'+Date.now();await d.save(name,undefined,false);
 const rows=[];
 for(const [name,color] of [['warm',[0.8,0.1,0.1]],['green',[0.1,0.8,0.1]],['cool',[0.1,0.1,0.8]]]){
  call('branch',{branch:name,base_revision:base.revision});
  const edit={branch:name,base_revision:base.revision,idempotency_key:'browser:variant:'+name,edits:{materials:[{material,base_color:color,emission:[0,0,0]}],light:settings.light,environment:[0.2,0.2,0.2]},max_added_bytes:1048576};
  const modified=call('modify',edit);check(JSON.stringify(call('modify',edit))===JSON.stringify(modified),'retry');
  if(name==='warm'){
   const promise=d.preview_gpu(name,modified.revision);check(d.cancel_preview(),'cancel acknowledgement');
   let cancelled=false;try{await promise;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
   let rejected=false;try{call('prepare_commit',{branch:name,revision:modified.revision,idempotency_key:'browser:cancelled:01',max_added_bytes:1048576});}catch(e){rejected=String(e).includes('preview_required');}check(rejected,'cancelled preview selection');
  }
  const cpu=call('preview',{branch:name,revision:modified.revision});
  const gpu=JSON.parse(await d.preview_gpu(name,modified.revision));
  check(gpu.protected_digest===base.protected_digest,'protected properties');
  check(gpu.inspection.visible_pixels>100,'visible geometry');
  const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);
  const mismatches=cpu.passes.object_ids.filter((v,i)=>v!==gpu.passes.object_ids[i]).length;
  check(rmse<0.02&&mismatches<=4,'CPU/WebGPU comparison');
  rows.push({name,revision:modified.revision,receipt:gpu.receipt,inspection:gpu.inspection,linear_rmse:rmse,object_mismatches:mismatches});
 }
 check(new Set(rows.map(r=>r.receipt.output_digest)).size===3,'distinct variants');
 const selected=rows.reduce((best,r)=>r.inspection.mean_linear_rgb[1]>best.inspection.mean_linear_rgb[1]?r:best);
 check(selected.name==='green','selection objective');
 const args={branch:selected.name,revision:selected.revision,idempotency_key:'browser:selection:01',max_added_bytes:1048576};
 const receipt=call('commit',args);check(JSON.stringify(call('commit',args))===JSON.stringify(receipt),'commit retry');
 await d.save(name,base.document_digest,true);
 let noOpBranch=m.BrowserAgent.import_json(d.export_json());
 const callOther=(method,args={})=>JSON.parse(noOpBranch.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 let otherBase=callOther('inspect');callOther('branch',{branch:'noop',base_revision:otherBase.revision});
 callOther('preview',{branch:'noop',revision:otherBase.revision});
 callOther('commit',{branch:'noop',revision:otherBase.revision,idempotency_key:'browser:no-op:0001',max_added_bytes:1048576});
 check(callOther('inspect').revision===otherBase.revision&&callOther('inspect').document_digest!==otherBase.document_digest,'retained receipt changes storage identity');

 let loaded=await m.BrowserAgent.load(name);check(JSON.parse(loaded.dispatch(JSON.stringify({version:0,operation:{method:'inspect'}}))).revision===base.revision,'aborted replacement');loaded.free();
 await d.save(name,base.document_digest,false);
 let conflicted=false;try{await d.save(name,base.document_digest,false);}catch(e){conflicted=String(e).includes('conflict');}check(conflicted,'storage conflict');
 await noOpBranch.save(name,otherBase.document_digest,false);
 let metadataConflict=false;try{await d.save(name,otherBase.document_digest,false);}catch(e){metadataConflict=String(e).includes('conflict');}check(metadataConflict,'same-revision receipt overwrite');
 const exported=noOpBranch.export_json();const restored=m.BrowserAgent.import_json(exported);
 check(restored.export_json()===exported,'export/import exact roundtrip');restored.free();
 globalThis.m05Agent=noOpBranch;globalThis.m05Name=name;globalThis.m05Revision=receipt.revision;globalThis.m05DocumentDigest=callOther('inspect').document_digest;d.free();
 return {status:'passed',profile:'gltf2-static-lambertian-v0',import:imported.report,variants:rows,chosen:selected.name,receipt,protected_digest:base.protected_digest,cancellation:true,aborted_save:true,save_conflict:true,same_revision_metadata_conflict:true,export_roundtrip:true,project:name,seconds:(performance.now()-started)/1000};
})()`);
await command('Storage.overrideQuotaForOrigin',{origin:'http://127.0.0.1:8765',quotaSize:1});
try {
 report.quota_failure = await evaluate(`(async()=>{try{await m05Agent.save(m05Name,m05DocumentDigest,false);throw new Error('expected quota failure');}catch(e){if(!String(e).includes('QuotaExceededError'))throw e;return true;}})()`);
} finally {await command('Storage.overrideQuotaForOrigin',{origin:'http://127.0.0.1:8765'});}
// Keep a replacement stream open, then terminate the page. Recovery must retain
// the previously closed project, including its selected light/material values.
await evaluate(`(async()=>{const root=await navigator.storage.getDirectory();const file=await root.getFileHandle('agent-'+m05Name+'.json');globalThis.m05Unclosed=await file.createWritable();await m05Unclosed.write('interrupted invalid replacement');return true;})()`);
await command('Page.reload',{ignoreCache:true});
await new Promise(resolve=>setTimeout(resolve,2000));
const recovery = await evaluate(`(async()=>{const m=await import('/pkg/render_web.js');const d=await m.BrowserAgent.load(${JSON.stringify(report.project)});const state=JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method:'inspect'}})));d.free();return {revision:state.revision,protected_digest:state.protected_digest,settings:state.snapshot.render_settings,branches:state.branches};})()`);
if(recovery.revision!==report.receipt.revision||recovery.protected_digest!==report.protected_digest||recovery.branches.length!==0)throw new Error('selected browser project recovery');
report.interrupted_stream_recovery = recovery;
report.browser = await command('Browser.getVersion');
await fs.mkdir('artifacts/m05',{recursive:true});
await fs.writeFile('artifacts/m05/browser_workflow_report.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({status:report.status,chosen:report.chosen,report:'artifacts/m05/browser_workflow_report.json'}));
} finally {socket.close();}
