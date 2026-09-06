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
 const d=m.BrowserAgent.import_json(${JSON.stringify(archive)}),native=${JSON.stringify(native)};
 const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const before=d.export_json(),state=call('inspect');check(state.revision===native.revision,'native revision');call('branch',{branch:'alpha',base_revision:state.revision});
 for(const [name,request] of [['nominal',native.nominal_request],['temporal',native.temporal_request]]){
   let error;try{await d.preview_frame_gpu('alpha',JSON.stringify(request));}catch(e){error=String(e);}check(error?.includes('budget'),'GPU '+name+' failure retained');
 }
 const valid=JSON.parse(await d.preview_frame_gpu('alpha',JSON.stringify(native.valid_request)));const cpu=call('preview_at',{branch:'alpha',request:native.valid_request});check(JSON.stringify(cpu.passes)===JSON.stringify(valid.passes),'valid CPU/GPU exact dyadic background');
 const delivered=[];let partial;try{await d.preview_sequence_gpu('alpha',JSON.stringify(native.sequence_request),(index,frame)=>{delivered.push(JSON.parse(frame));check(index===0,'only preceding complete frame');});}catch(e){partial=JSON.parse(String(e));}
 check(partial?.delivered_frames===1&&partial.acknowledged_frames===1&&partial.error.includes('budget'),'typed partial failure');check(delivered.length===1,'one frame delivered');
 const pending=d.preview_frame_gpu('alpha',JSON.stringify(native.valid_request));d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'frame cancellation');
 const project='gpu-alpha-shutter-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS exact recovery');
 const rcall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));rcall('branch',{branch:'alpha',base_revision:state.revision});const again=JSON.parse(await restored.preview_frame_gpu('alpha',JSON.stringify(native.valid_request)));check(JSON.stringify(again.passes)===JSON.stringify(valid.passes),'recovered GPU frame');check(d.export_json()===before,'immutable root');
 restored.free();d.free();return {status:'passed',nominal_failure_retained:true,temporal_failure_retained:true,partial,cancelled,opfs_restored:true};
 })()`);
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
