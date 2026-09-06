// Independent CDP client; authored state, evaluation, transport and OPFS are Rust.
import fs from 'node:fs/promises';
const root=process.argv[2]??'artifacts/m07';
const targets=await(await fetch('http://127.0.0.1:9223/json/list')).json();
const socket=new WebSocket(targets.find(x=>x.type==='page').webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
let sequence=0;const pending=new Map();socket.onmessage=event=>{const m=JSON.parse(event.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);if(m.error)p.reject(m.error);else p.resolve(m.result);}};
function command(method,params={}){const id=++sequence;return new Promise((resolve,reject)=>{pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await command('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
try {
 await command('Page.enable');await command('Runtime.enable');await command('Network.enable');await command('Network.setCacheDisabled',{cacheDisabled:true});await command('Page.navigate',{url:'http://127.0.0.1:8765/'});
 await evaluate(`new Promise((resolve,reject)=>{const deadline=Date.now()+120000;const poll=()=>{const e=document.getElementById('report');if(e?.dataset.status==='passed')resolve(true);else if(e?.dataset.status==='failed')reject(new Error(e.textContent));else if(Date.now()>deadline)reject(new Error('page timeout'));else setTimeout(poll,100);};poll();})`);
 const rows=[];
 for (const name of (process.argv[3]??'geometry,volume,groom').split(',')) {
  const document=await fs.readFile(`${root}/${name}-01/document.json`,'utf8');
  const row=await evaluate(`(async()=>{
   const m=await import('/pkg/render_web.js');const check=(x,s)=>{if(!x)throw new Error(s);};
   const d=m.BrowserAgent.import_json(${JSON.stringify(document)});const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
   const state=call('inspect');const project='m07-${name}-'+Date.now();await d.save(project,undefined,false);const loaded=await m.BrowserAgent.load(project);check(loaded.export_json()===d.export_json(),'OPFS exact typed asset recovery');loaded.free();
   call('branch',{branch:'authored',base_revision:state.revision});let stale=false;try{call('preview',{branch:'authored',revision:'stale'});}catch(e){stale=String(e).includes('conflict');}check(stale,'stale preview rejected');
   const at=performance.now();const cpu=call('preview',{branch:'authored',revision:state.revision});const cpu_seconds=(performance.now()-at)/1000;
   let gpu=null,rejection=null,cancelled=null;
   if(${JSON.stringify(name)}!=='volume') {
    const promise=d.preview_gpu('authored',state.revision);check(d.cancel_preview(),'cancel acknowledgement');cancelled=false;try{await promise;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'geometry GPU cancellation');
    gpu=JSON.parse(await d.preview_gpu('authored',state.revision));
   } else {try{await d.preview_gpu('authored',state.revision);}catch(e){rejection=String(e);}check(rejection?.includes('sparse media'),'GPU volume explicit rejection');}
   const rmse=(a,b)=>{a=a.flat();b=b.flat();return Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);};
   const linear_rmse=gpu?rmse(cpu.passes.linear_rgb,gpu.passes.linear_rgb):null,normal_rmse=gpu?rmse(cpu.passes.normals_world,gpu.passes.normals_world):null;
   const object_mismatches=gpu?cpu.passes.object_ids.filter((x,i)=>x!==gpu.passes.object_ids[i]).length:null;
   if(gpu)check(linear_rmse<0.025&&normal_rmse<0.025&&object_mismatches<=4,'geometry WebGPU equivalence');
   const result={name:${JSON.stringify(name)},revision:state.revision,cpu_seconds,linear_rmse,normal_rmse,object_mismatches,gpu_rejection:rejection,cancelled,stale_rejected:stale,opfs_roundtrip:true,cpu_receipt:cpu.receipt,gpu_receipt:gpu?.receipt,passes:cpu.passes};d.free();return result;
  })()`);
  await fs.writeFile(`${root}/${name}-browser-passes.json`,JSON.stringify(row.passes)+'\n');delete row.passes;rows.push(row);console.log(JSON.stringify(row));
 }
 await fs.writeFile(`${root}/browser_report.json`,JSON.stringify({status:'passed',browser:await command('Browser.getVersion'),workflows:rows},null,2)+'\n');
}finally{socket.close();}
