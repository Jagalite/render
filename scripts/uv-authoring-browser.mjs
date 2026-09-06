// Independent UV authoring client; geometry, transactions and rendering are Rust.
import fs from 'node:fs/promises';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8780';
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
 const at=performance.now();const initial=new m.BrowserDocument('000000000000000000000000000003b6');
 check(initial.revision()===native.create.base_revision,'initial revision');initial.execute(JSON.stringify(native.create));
 const d=m.BrowserAgent.import_json(initial.export_json());initial.free();
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const results=[];
 for(const step of native.operations){
  const result=call('author_uv',{request:step.operation.request});
  check(JSON.stringify(call('author_uv',{request:step.operation.request}))===JSON.stringify(result),'idempotent UV retry');
  check(result.report.output_mesh===step.result.report.output_mesh,'native/Wasm mesh identity');
  check(result.uv_asset===step.result.uv_asset,'native/Wasm constraints identity');
  check(result.receipt.revision===step.result.receipt.revision,'native/Wasm transaction revision');
  results.push(result);
 }
 const state=call('inspect'),before=d.export_json();check(state.revision===native.revision,'final revision');
 const stale={...native.operations[0].operation.request,idempotency_key:'uv-browser-stale-request'};
 let staleError=false;try{call('author_uv',{request:stale});}catch(e){staleError=String(e).includes('stale_revision');}check(staleError,'stale authoring rejected');
 let budgetError=false;try{call('author_uv',{request:{...stale,budget:{max_output_bytes:1,max_solver_work:16777216}}});}catch(e){budgetError=String(e).includes('budget');}check(budgetError,'output budget rejected');
 check(d.export_json()===before,'failed UV requests atomic');
 const project='uv-authoring-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS authoring constraints');
 call('branch',{branch:'uv-output',base_revision:state.revision});
 const cpu=call('preview',{branch:'uv-output',revision:state.revision});const gpu=JSON.parse(await d.preview_gpu('uv-output',state.revision));
 const pending=d.preview_gpu('uv-output',state.revision);d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
 const restoredCall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 restoredCall('branch',{branch:'uv-restored',base_revision:state.revision});const again=restoredCall('preview',{branch:'uv-restored',revision:state.revision});
 check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'recovered CPU render');check(d.export_json()===before,'preview source immutable');
 const result={status:'passed',results,revision:state.revision,opfs_restored:true,stale_rejected:staleError,budget_rejected:budgetError,gpu_cancelled:cancelled,cpu:cpu.passes,gpu:gpu.passes,seconds:(performance.now()-at)/1000};restored.free();d.free();return result;
 })()`);
 for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-box-'+backend+'.json',JSON.stringify(report[backend])+'\n');delete report[backend];}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
