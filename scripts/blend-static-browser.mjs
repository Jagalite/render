// Independent browser client. All import, storage, evaluation and rendering is Rust.
import fs from 'node:fs/promises';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8779';
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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));const rows=[];
 for(const row of native.variants)rows.push({...row,bytes:[...await fs.readFile(row.source.path)]});
 const results=[];
 for(const row of rows){
  const result=await evaluate(`(async()=>{
   const row=${JSON.stringify(row)};const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(row.name+': '+s);};
   const d=new m.BrowserAgent('${(293).toString(16).padStart(32,'0')}');
   const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
   const at=performance.now();const empty=call('inspect');check(empty.revision===row.empty_revision,'empty revision');
   const request={bytes:row.bytes,policy:row.source.policy,settings:row.settings,base_revision:empty.revision,idempotency_key:'blend:import:0001'};
   const imported=call('import_blend',request);check(JSON.stringify(imported.report)===JSON.stringify(row.report),'same conversion report');
   const state=call('inspect');check(state.revision===row.revision,'same native revision: '+state.revision+' versus '+row.revision);
   check(JSON.stringify(call('import_blend',request))===JSON.stringify(imported),'idempotent retry');
   const before=d.export_json();const source=call('export_source',{request:{revision:row.revision,asset:row.report.source_asset}});check(JSON.stringify(source.bytes)===JSON.stringify(row.bytes),'exact original source');
   call('branch',{branch:'blend',base_revision:row.revision});const cpu=call('preview',{branch:'blend',revision:row.revision});const gpu=JSON.parse(await d.preview_gpu('blend',row.revision));
   check(cpu.passes.object_ids.some(x=>x!==null),'visible imported geometry');
   let stale=false;try{call('export_source',{request:{revision:row.empty_revision,asset:row.report.source_asset}});}catch(e){stale=String(e).includes('stale_revision');}check(stale,'stale source export rejected');
   let invalid=false;const bytes=[...row.bytes];bytes[9]=50;bytes[10]=57;bytes[11]=50;try{call('import_blend',{...request,bytes,idempotency_key:'blend:invalid:001'});}catch(e){invalid=String(e).includes('unsupported_blend');}check(invalid,'unsupported source version rejected');
   const work=d.preview_gpu('blend',row.revision);d.cancel_preview();let cancelled=false;try{await work;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
   check(d.export_json()===before,'read/failure root unchanged');
   const project='blend-static-'+row.name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS exact archive');
   const again=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
   const recovered=again('export_source',{request:{revision:row.revision,asset:row.report.source_asset}});check(JSON.stringify(recovered.bytes)===JSON.stringify(row.bytes),'OPFS exact source');
   again('branch',{branch:'blend',base_revision:row.revision});const cpu2=again('preview',{branch:'blend',revision:row.revision});const gpu2=JSON.parse(await restored.preview_gpu('blend',row.revision));
   check(JSON.stringify(cpu2.passes)===JSON.stringify(cpu.passes),'recovered CPU passes');check(JSON.stringify(gpu2.passes)===JSON.stringify(gpu.passes),'recovered GPU passes');
   const out={name:row.name,revision:state.revision,source_digest:source.source_digest,source_bytes:source.bytes.length,opfs_restored:true,seconds:(performance.now()-at)/1000,cpu:cpu.passes,gpu:gpu.passes};restored.free();d.free();return out;
  })()`);
  for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+row.name+'-'+backend+'.json',JSON.stringify(result[backend])+'\n');delete result[backend];}
  results.push(result);console.log(row.name+': passed');
 }
 const report={status:'passed',results,browser:await command('Browser.getVersion')};await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');
} finally {socket.close();}
