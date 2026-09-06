// Independent browser client; import, evaluation, storage and rendering are Rust.
import fs from 'node:fs/promises';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8766';
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
 const frames=[];
 for(const depth of [1,2,4,16]) {
 const document=await fs.readFile(root+'/archive-'+depth+'/document/document.json','utf8');
 const frame=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};
 const d=m.BrowserAgent.import_json(${JSON.stringify(document)});
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const revision=call('inspect').revision;const before=d.export_json();call('branch',{branch:'transport',base_revision:revision});
 const at=performance.now();
 const cpu=call('preview',{branch:'transport',revision});
 const gpu=JSON.parse(await d.preview_gpu('transport',revision));
 const x=cpu.passes.linear_rgb.flat(),y=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(x.reduce((s,v,i)=>s+(v-y[i])**2,0)/x.length);
 check(rmse<.002,'CPU/WebGPU RMSE '+rmse);check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'object parity');
 let stale=false;try{await d.preview_gpu('transport','stale');}catch(e){stale=String(e).includes('conflict');}check(stale,'stale rejection');
 const pending=d.preview_gpu('transport',revision);check(d.cancel_preview(),'cancel ack');let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
 const project='multibounce-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS recovery');restored.free();check(d.export_json()===before,'immutable authored state');d.free();
 return {depth:${depth},seconds:(performance.now()-at)/1000,linear_rmse:rmse,stale_rejected:stale,cancelled,opfs_roundtrip:true,cpu:cpu.passes,gpu:gpu.passes};
 })()`);
 for(const backend of ['cpu','gpu']) {await fs.writeFile(root+'/browser-'+backend+'-'+depth+'.json',JSON.stringify(frame[backend])+'\n');delete frame[backend];}
 frames.push(frame);
 }
 const report={status:'passed',frames,browser:await command('Browser.getVersion')};await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
