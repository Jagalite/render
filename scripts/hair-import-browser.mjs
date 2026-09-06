// Independent browser client; import, evaluation, storage and rendering are Rust.
import fs from 'node:fs/promises';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8770';
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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));const cases=[];
 for(const row of native.variants)cases.push({...row,bytes:[...await fs.readFile(row.source.path)],source_format:'hair'});
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const at=performance.now();const results=[];
 for(const row of ${JSON.stringify(cases)}){
  const d=new m.BrowserAgent('${(10200).toString(16).padStart(32,'0')}');
  const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  const empty=call('inspect');check(empty.revision===row.empty_revision,'same empty revision');
  const request={bytes:row.bytes,policy:row.source.policy,settings:row.settings,base_revision:empty.revision,idempotency_key:'hair:import:0001'};
  const imported=call('import_hair',request);check(JSON.stringify({...imported.report,observed_derived_json_bytes:undefined})===JSON.stringify({...row.report,observed_derived_json_bytes:undefined}),'same typed import report '+JSON.stringify(Object.fromEntries(Object.keys(row.report).filter(k=>JSON.stringify(imported.report[k])!==JSON.stringify(row.report[k])).map(k=>[k,{native:row.report[k],browser:imported.report[k]}]))));
  check(call('inspect').revision===row.revision,'same native revision');call('import_hair',request);
  const evaluated=call('render_root_cpu',{revision:row.revision});check(evaluated.conversions.reduce((sum,c)=>sum+c.derived_bytes,0)===imported.report.observed_derived_json_bytes,'measured derived JSON bytes match actual evaluation');
  const before=d.export_json();call('branch',{branch:'hair',base_revision:row.revision});
  const cpu=call('preview',{branch:'hair',revision:row.revision});check(JSON.stringify(cpu.passes)===JSON.stringify(evaluated.passes),'root/branch render agreement');const gpu=JSON.parse(await d.preview_gpu('hair',row.revision));const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((sum,x,i)=>sum+(x-b[i])**2,0)/a.length);check(rmse<.002,'hair CPU/WebGPU parity '+rmse);check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'objects');
  const pending=d.preview_gpu('hair',row.revision);d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
  let stale=false;try{call('import_hair',{...request,idempotency_key:'hair:stale:00001'});}catch(e){stale=String(e).includes('import_target');}check(stale,'stale nonempty import rejected');
  let invalid=false;const bad=[...row.bytes];bad[12]=34;try{call('import_hair',{...request,bytes:bad,base_revision:row.revision,idempotency_key:'hair:invalid:001'});}catch(e){invalid=String(e).includes('unsupported_hair');}check(invalid,'unsupported HAIR rejected');check(d.export_json()===before,'failure root immutable');
  const project='hair-import-'+row.name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS exact recovery');
  const loaded=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));loaded('branch',{branch:'hair',base_revision:row.revision});const again=loaded('preview',{branch:'hair',revision:row.revision});check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'recovered CPU image');
  results.push({name:row.name,opfs_restored:true,cpu:cpu.passes,gpu:gpu.passes,report:imported.report});restored.free();d.free();
 }
 return {status:'passed',results,seconds:(performance.now()-at)/1000};
 })()`);
 for(const row of report.results){for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+row.name+'-'+backend+'.json',JSON.stringify(row[backend])+'\n');delete row[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
