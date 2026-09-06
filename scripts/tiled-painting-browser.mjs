// Independent tiled painting client; geometry, transactions and rendering are Rust.
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8781';
const cdp=process.env.RENDER_TEST_CDP??'http://127.0.0.1:9224';
const targets=await(await fetch(cdp+'/json/list')).json();
const socket=new WebSocket(targets.find(x=>x.type==='page').webSocketDebuggerUrl);
await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j;});
let sequence=0;const pending=new Map();
socket.onmessage=e=>{const m=JSON.parse(e.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result);}};
const command=(method,params={})=>{const id=++sequence;return new Promise((resolve,reject)=>{pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});};
const evaluate=async expression=>{const r=await command('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;};
try {
 await command('Page.enable');await command('Network.enable');await command('Network.setCacheDisabled',{cacheDisabled:true});await command('Page.navigate',{url:origin+'/'});
 await evaluate(`new Promise((r,j)=>{const until=Date.now()+120000;const poll=()=>{const e=document.getElementById('report');if(e?.dataset.status==='passed')r(true);else if(e?.dataset.status==='failed'||Date.now()>until)j(new Error(e?.textContent??'timeout'));else setTimeout(poll,100);};poll();})`);
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const native=${JSON.stringify(native)};const check=(v,s)=>{if(!v)throw new Error(s);};
 const at=performance.now();let d=new m.BrowserAgent(native.document_id);
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const results=[];
 for(const step of native.operations){
  let result;
  if(step.operation.method==='apply') {
   const generic=m.BrowserDocument.import_json(d.export_json());
   result={receipt:JSON.parse(generic.execute(JSON.stringify(step.operation.request)))};
   d.free();d=m.BrowserAgent.import_json(generic.export_json());generic.free();
  } else {
   result=call(step.operation.method,{request:step.operation.request});
   check(JSON.stringify(call(step.operation.method,{request:step.operation.request}))===JSON.stringify(result),'idempotent authoring retry');
   check(JSON.stringify(result.report)===JSON.stringify(step.result.report),'native/Wasm numeric reports and asset identities');
  }
  check(result.receipt.revision===step.result.receipt.revision,'native/Wasm transaction revision');
  results.push(result);
 }
 const state=call('inspect'),before=d.export_json();check(JSON.stringify(state.snapshot)===JSON.stringify(native.snapshot),'native/Wasm full snapshot and PNG bytes');check(state.revision===native.revision,'final revision');
 const stale={...native.operations[4].operation.request,idempotency_key:'paint-browser-stale-request'};
 let staleError=false;try{call('author_paint',{request:stale});}catch(e){staleError=String(e).includes('stale_revision');}check(staleError,'stale authoring rejected');
 let budgetError=false;try{call('author_paint',{request:{...stale,budget:{...stale.budget,max_output_bytes:1}}});}catch(e){budgetError=String(e).includes('budget');}check(budgetError,'output budget rejected');
 check(d.export_json()===before,'failed painting requests atomic');
 const project='tiled-painting-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS paint layers and tiles');
 call('branch',{branch:'paint-output',base_revision:state.revision});
 const cpu=call('preview',{branch:'paint-output',revision:state.revision});const gpu=JSON.parse(await d.preview_gpu('paint-output',state.revision));
 const pending=d.preview_gpu('paint-output',state.revision);d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
 const restoredCall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 restoredCall('branch',{branch:'paint-restored',base_revision:state.revision});const again=restoredCall('preview',{branch:'paint-restored',revision:state.revision});
 check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'recovered CPU render');check(d.export_json()===before,'preview source immutable');
 const wasmBytes=await(await fetch('/pkg/render_web_bg.wasm')).arrayBuffer();const package_sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',wasmBytes)),v=>v.toString(16).padStart(2,'0')).join('');
 const result={status:'passed',package_sha256,results,revision:state.revision,opfs_restored:true,stale_rejected:staleError,budget_rejected:budgetError,gpu_cancelled:cancelled,cpu:cpu.passes,gpu:gpu.passes,seconds:(performance.now()-at)/1000};restored.free();d.free();return result;
 })()`);
 for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-box-'+backend+'.json',JSON.stringify(report[backend])+'\n');delete report[backend];}
 if(report.package_sha256!==createHash('sha256').update(await fs.readFile('web/pkg/render_web_bg.wasm')).digest('hex'))throw new Error('served Wasm package mismatch');
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
