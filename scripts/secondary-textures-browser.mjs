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
 for(const row of native.variants)for(const filter of row.filters)cases.push({name:row.name,gpu:row.gpu,filter:filter.name,revision:filter.revision,archive:await fs.readFile(filter.archive,'utf8')});
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const at=performance.now();const results=[],previous={};
 for(const row of ${JSON.stringify(cases)}){
  const d=m.BrowserAgent.import_json(row.archive);const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  const before=d.export_json(),state=call('inspect');check(state.revision===row.revision,'same native revision');call('branch',{branch:'surface',base_revision:state.revision});
  const cpu=call('preview',{branch:'surface',revision:state.revision});let gpu=null;
  if(row.gpu){gpu=JSON.parse(await d.preview_gpu('surface',state.revision));const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);check(rmse<.002,'CPU/WebGPU parity '+rmse);}
  else{let unsupported=false;try{await d.preview_gpu('surface',state.revision);}catch(e){unsupported=String(e).includes('unsupported_profile');}check(unsupported,'richer GPU BSDF remains explicit');}
  if(previous[row.name]){check(JSON.stringify(previous[row.name].cpu)===JSON.stringify(cpu.passes),'secondary CPU sampler invariant');if(gpu)check(JSON.stringify(previous[row.name].gpu)===JSON.stringify(gpu.passes),'secondary GPU sampler invariant');}
  else previous[row.name]={cpu:cpu.passes,gpu:gpu?.passes??null};
  const project='secondary-texture-'+row.name+'-'+row.filter+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS exact recovery');
  const loaded=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));loaded('branch',{branch:'surface',base_revision:state.revision});const again=loaded('preview',{branch:'surface',revision:state.revision});check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'recovered CPU image');check(d.export_json()===before,'immutable root');
  results.push({name:row.name,filter:row.filter,opfs_restored:true,cpu:cpu.passes,gpu:gpu?.passes??null});restored.free();d.free();
 }
 return {status:'passed',results,seconds:(performance.now()-at)/1000};
 })()`);
 for(const row of report.results){for(const backend of ['cpu','gpu']){if(row[backend])await fs.writeFile(root+'/browser-'+row.name+'-'+row.filter+'-'+backend+'.json',JSON.stringify(row[backend])+'\n');delete row[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
