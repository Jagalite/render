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
 for(const row of native.variants)cases.push({...row,archive:await fs.readFile(row.archive,'utf8')});
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const at=performance.now();const results=[];
 const dispatch=d=>(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 for(const row of ${JSON.stringify(cases)}){
  const d=m.BrowserAgent.import_json(row.archive);const call=dispatch(d);
  check(call('inspect').revision===row.revision,'native revision');const before=d.export_json();call('branch',{branch:'media',base_revision:row.revision});
  const cpu=call('preview',{branch:'media',revision:row.revision});const gpu=JSON.parse(await d.preview_gpu('media',row.revision));
  const errors={};for(const [name,image] of [['cpu',cpu],['gpu',gpu]]){errors[name]=Math.max(...image.passes.linear_rgb.flat().map((x,i)=>Math.abs(x-row.expected[i])));check(errors[name]<(name==='cpu'?2e-6:2e-5),'analytic '+row.name+' '+name+' '+errors[name]);}
  check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'objects');
  const pending=d.preview_gpu('media',row.revision);d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
  const stale=(row.revision[0]==='0'?'1':'0')+row.revision.slice(1);let rejected=false;try{await d.preview_gpu('media',stale);}catch(e){rejected=String(e).includes('revision');}check(rejected,'GPU stale revision');
  const project='gpu-media-'+row.name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS recovery');const loaded=dispatch(restored);loaded('branch',{branch:'media',base_revision:row.revision});
  check(JSON.stringify(loaded('preview',{branch:'media',revision:row.revision}).passes)===JSON.stringify(cpu.passes),'recovered CPU pixels');const again=JSON.parse(await restored.preview_gpu('media',row.revision));check(JSON.stringify(again.passes)===JSON.stringify(gpu.passes),'recovered GPU pixels');check(d.export_json()===before,'immutable root');
  results.push({name:row.name,opfs_restored:true,stale_rejected:true,cancelled:true,max_absolute_error:errors,cpu:cpu.passes,gpu:gpu.passes});restored.free();d.free();
 }
 return {status:'passed',results,seconds:(performance.now()-at)/1000};
 })()`);
 for(const row of report.results){for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+row.name+'-'+backend+'.json',JSON.stringify(row[backend])+'\n');delete row[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
