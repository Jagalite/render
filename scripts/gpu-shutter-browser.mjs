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
 const archive=await fs.readFile(root+'/archive/document/document.json','utf8');
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};
 const d=m.BrowserAgent.import_json(${JSON.stringify(archive)});
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const at=performance.now();const state=call('inspect');const before=d.export_json();
 const clip=${JSON.stringify(native.clip)},revision=state.revision;
 const project='gpu-shutter-'+Date.now();await d.save(project,undefined,false);
 call('branch',{branch:'clip',base_revision:revision});
 const request={revision,clip,time:{numerator:1,denominator:1},shutter:{open:{numerator:-1,denominator:32},close:{numerator:1,denominator:32},samples:3}};
 let cancelled=false;const pending=d.preview_frame_gpu('clip',JSON.stringify(request));check(d.cancel_preview(),'immediate cancel acknowledged');try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'immediate cancel rejected');
 const cpu=call('preview_at',{branch:'clip',request});
 const gpu=JSON.parse(await d.preview_frame_gpu('clip',JSON.stringify(request)));
 const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);
 check(rmse<.002,'CPU/WebGPU shutter parity');check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'nominal object IDs');
 let unqualified=false;try{call('commit',{branch:'clip',revision,idempotency_key:'shutter:unqualified:01',max_added_bytes:8388608});}catch(e){unqualified=String(e).includes('preview_required');}check(unqualified,'animated preview must not qualify static commit');
 const sequence={revision,clip,times:[{numerator:0,denominator:1},{numerator:1,denominator:1},{numerator:2,denominator:1}],shutter:request.shutter};
 const frames=[];let consumerBusy=false;
 const complete=JSON.parse(await d.preview_sequence_gpu('clip',JSON.stringify(sequence),async(i,json)=>{
   check(!consumerBusy,'consumer backpressure');consumerBusy=true;await new Promise(r=>setTimeout(r,10));frames.push(JSON.parse(json));check(i===frames.length-1,'ordered delivery');consumerBusy=false;
 }));
 check(complete.status==='complete'&&complete.frames===3,'sequence completed');
 check(frames[1].receipt.output_digest===gpu.receipt.output_digest,'frame/sequence identity');
 let partial;try{await d.preview_sequence_gpu('clip',JSON.stringify(sequence),()=>{d.cancel_preview();});}catch(e){partial=JSON.parse(String(e));}
 check(partial?.acknowledged_frames===1&&partial.delivered_frames===1&&partial.error.includes('cancel'),'partial cancellation');
 let sinkFailure;try{await d.preview_sequence_gpu('clip',JSON.stringify(sequence),()=>Promise.reject('sink rejected'));}catch(e){sinkFailure=JSON.parse(String(e));}
 check(sinkFailure?.delivered_frames===1&&sinkFailure.acknowledged_frames===0&&sinkFailure.error.includes('sink rejected'),'consumer rejection');
 let invalid=false;try{d.preview_sequence_gpu('clip',JSON.stringify({...sequence,times:[sequence.times[1],sequence.times[0]]}),()=>{throw new Error('unexpected delivery');});}catch(e){invalid=String(e).includes('time');}check(invalid,'invalid sequence order');
 call('branch',{branch:'sequence-stale',base_revision:revision});
 let staleSequence;try{await d.preview_sequence_gpu('sequence-stale',JSON.stringify(sequence),()=>{
   call('modify',{branch:'sequence-stale',base_revision:revision,edits:{materials:[],environment:[.2,.2,.2],light:state.snapshot.render_settings.light},idempotency_key:'shutter:sequence:stale',max_added_bytes:8388608});
 });}catch(e){staleSequence=JSON.parse(String(e));}
 check(staleSequence?.acknowledged_frames===1&&staleSequence.error.includes('conflict'),'stale sequence stopped after acknowledged frame');
 // Mutate the branch while the async frame is pending, via the ordinary agent API.
 const stalePromise=d.preview_frame_gpu('clip',JSON.stringify(request));
 const light=state.snapshot.render_settings.light;
 call('modify',{branch:'clip',base_revision:revision,edits:{materials:[],environment:[.2,.2,.2],light},idempotency_key:'shutter:stale:01',max_added_bytes:8388608});
 let stale=false;try{await stalePromise;}catch(e){stale=String(e).includes('conflict');}check(stale,'stale GPU completion');
 const restored=await m.BrowserAgent.load(project);const restoredCall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 restoredCall('branch',{branch:'clip',base_revision:revision});
 const restoredFrame=JSON.parse(await restored.preview_frame_gpu('clip',JSON.stringify(request)));
 check(restoredFrame.receipt.output_digest===gpu.receipt.output_digest,'OPFS restored frame');check(d.export_json()===before,'authored root unchanged');
 restored.free();d.free();return {status:'passed',profile:'gpu-f32-animation-v0',revision,linear_rmse:rmse,partial,sinkFailure,staleSequence,stale_rejected:stale,immediate_cancelled:cancelled,opfs_roundtrip:true,consumer_backpressure:true,frames:complete.frames,cpu:cpu.passes,gpu:gpu.passes,seconds:(performance.now()-at)/1000};
 })()`);
 for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+backend+'.json',JSON.stringify(report[backend])+'\n');delete report[backend];}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
