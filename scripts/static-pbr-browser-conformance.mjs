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
const rows=[];
for(const [index,name] of ['BoxTextured','NormalTangentMirrorTest'].entries()) {
 const asset=(await fs.readFile(`fixtures/static-pbr/${name}/${name}.glb`)).toString('base64');
 const row=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(x,s)=>{if(!x)throw new Error(s);};
 const d=new m.BrowserAgent(${JSON.stringify((800+index).toString(16).padStart(32,'0'))});
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const settings={width:64,height:64,samples:32,seed:42,max_depth:1,max_bytes:268435456,environment:[0.25,0.25,0.25],light:{position:[2,3,4],intensity:[20,20,20]},camera:{position:${index===0?'[2,1.5,3]':'[0,0,5]'},target:[0,0,0],up:[0,1,0],vertical_fov_radians:0.6}};
 const started=performance.now();
 const imported=call('import_pbr_glb',{bytes:Array.from(atob(${JSON.stringify(asset)}),c=>c.charCodeAt(0)),policy:{allow_approximations:true},settings,base_revision:call('inspect').revision,idempotency_key:${JSON.stringify('pbr:import:'+name+':01')}});
 const state=call('inspect');const project='pbr-${name}-'+Date.now();await d.save(project,undefined,false);
 const loaded=await m.BrowserAgent.load(project);check(loaded.export_json()===d.export_json(),'OPFS exact PBR roundtrip');loaded.free();
 const restored=m.BrowserAgent.import_json(d.export_json());check(restored.export_json()===d.export_json(),'encoded-image export roundtrip');restored.free();
 call('branch',{branch:'asset',base_revision:state.revision});
 const promise=d.preview_gpu('asset',state.revision);check(d.cancel_preview(),'cancel acknowledgement');let cancelled=false;try{await promise;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'PBR WebGPU cancellation');
 let stale=false;try{call('preview',{branch:'asset',revision:'stale'});}catch(e){stale=String(e).includes('conflict');}check(stale,'stale PBR preview');
 const at=performance.now();const cpu=call('preview',{branch:'asset',revision:state.revision});const cpu_seconds=(performance.now()-at)/1000;
 const ag=performance.now();const gpu=JSON.parse(await d.preview_gpu('asset',state.revision));const gpu_seconds=(performance.now()-ag)/1000;
 const rmse=(a,b)=>{a=a.flat();b=b.flat();return Math.sqrt(a.reduce((sum,x,i)=>sum+(x-b[i])**2,0)/a.length);};
 const linear_rmse=rmse(cpu.passes.linear_rgb,gpu.passes.linear_rgb),normal_rmse=rmse(cpu.passes.normals_world,gpu.passes.normals_world);
 const object_mismatches=cpu.passes.object_ids.filter((id,i)=>id!==gpu.passes.object_ids[i]).length;
 check(linear_rmse<0.025&&normal_rmse<0.025&&object_mismatches<=4&&gpu.inspection.visible_pixels>100,'PBR CPU/WebGPU comparison');
 const result={asset:${JSON.stringify(name)},revision:state.revision,import:imported.report,linear_rmse,normal_rmse,object_mismatches,visible_pixels:gpu.inspection.visible_pixels,cpu_seconds,gpu_seconds,seconds:(performance.now()-started)/1000,receipt:gpu.receipt,cpu_receipt:cpu.receipt,opfs_roundtrip:true,cancelled,stale_rejected:stale,passes:gpu.passes};d.free();return result;
})()`);
 await fs.mkdir('artifacts/static-pbr',{recursive:true});
 await fs.writeFile(`artifacts/static-pbr/${name}-browser-passes.json`,JSON.stringify(row.passes)+'\n');delete row.passes;rows.push(row);console.log(JSON.stringify({asset:name,linear_rmse:row.linear_rmse,normal_rmse:row.normal_rmse}));
}
const report={status:'passed',profile:'gltf2-static-pbr-v0',browser:await command('Browser.getVersion'),assets:rows};
await fs.writeFile('artifacts/static-pbr/browser_workflow_report.json',JSON.stringify(report,null,2)+'\n');
} finally {socket.close();}
