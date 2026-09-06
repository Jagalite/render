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
 const asset=(await fs.readFile('fixtures/animated-gltf/character.glb')).toString('base64');
 const requests=await fs.readdir(root+'/requests');const input=JSON.parse(await fs.readFile(root+'/requests/'+requests.find(n=>n.endsWith('-import.json')),'utf8'));
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};
 const d=new m.BrowserAgent('00000000000000000000000000000063');
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const at=performance.now();const imported=call('import_pbr_glb',{bytes:Array.from(atob(${JSON.stringify(asset)}),c=>c.charCodeAt(0)),policy:{allow_approximations:true},settings:${JSON.stringify(input.operation.settings)},base_revision:call('inspect').revision,idempotency_key:'browser:animated:01'});
 const state=call('inspect');const before=d.export_json();const clip=imported.report.source_clips['0'];
 const project='animated-gltf-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS roundtrip');restored.free();
 call('branch',{branch:'clip',base_revision:state.revision});const frames=[];
 for(const [n,den] of [[0,1],[1,1],[3,2],[1,1]]) {
 const f=call('preview_at',{branch:'clip',request:{revision:state.revision,clip,time:{numerator:n,denominator:den},shutter:{open:{numerator:-1,denominator:32},close:{numerator:1,denominator:32},samples:3}}});
 check(f.inspection.visible_pixels>10,'animated frame visible');frames.push({n,den,receipt:f.receipt,passes:f.passes});
 }
 check(frames[1].receipt.output_digest===frames[3].receipt.output_digest,'random access stable');
 let stale=false;try{call('preview_at',{branch:'clip',request:{revision:'stale',clip,time:{numerator:1,denominator:1},shutter:{open:{numerator:0,denominator:1},close:{numerator:0,denominator:1},samples:1}}});}catch(e){stale=String(e).includes('conflict');}check(stale,'stale revision');
 const promise=d.preview_gpu('clip',state.revision);check(d.cancel_preview(),'cancel ack');let cancelled=false;try{await promise;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
 const cpu=call('preview',{branch:'clip',revision:state.revision});const gpu=JSON.parse(await d.preview_gpu('clip',state.revision));
 const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);const mismatch=cpu.passes.object_ids.filter((x,i)=>x!==gpu.passes.object_ids[i]).length;
 check(rmse<.025&&mismatch<=4,'default pose CPU/WebGPU parity');check(d.export_json()===before,'authored state unchanged');
 const result={status:'passed',profile:'gltf2-animated-pbr-v0',revision:state.revision,import:imported.report,opfs_roundtrip:true,random_access:true,stale_rejected:stale,gpu_cancelled:cancelled,default_pose_gpu:{linear_rmse:rmse,object_mismatches:mismatch,passes:gpu.passes},frames,seconds:(performance.now()-at)/1000};d.free();return result;
 })()`);
 for(let i=0;i<report.frames.length;i++){await fs.writeFile(root+'/browser-frame-'+i+'.json',JSON.stringify(report.frames[i].passes)+'\n');delete report.frames[i].passes;}
 await fs.writeFile(root+'/browser-default-gpu.json',JSON.stringify(report.default_pose_gpu.passes)+'\n');delete report.default_pose_gpu.passes;
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({status:report.status,seconds:report.seconds,default_pose_gpu:report.default_pose_gpu}));
} finally {socket.close();}
