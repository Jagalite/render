// Independent browser client; import, evaluation, storage and rendering are Rust.
import fs from 'node:fs/promises';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8768';
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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));const variants=[];
 for(const name of ['dense','sparse'])variants.push({name,asset:(await fs.readFile('fixtures/morph-frames/data/'+name+'.glb')).toString('base64')});
 const bad=JSON.parse(await fs.readFile(root+'/malformed.gltf','utf8'));const bin=(await fs.readFile('fixtures/morph-frames/data/dense.bin')).toString('base64');
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const settings=${JSON.stringify(native.settings)};const at=performance.now();const results=[];
 for(const {name,asset} of ${JSON.stringify(variants)}){
  const d=new m.BrowserAgent('00000000000000000000000000002710');const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  const args={bytes:Array.from(atob(asset),c=>c.charCodeAt(0)),policy:{allow_approximations:true},settings,base_revision:call('inspect').revision,idempotency_key:'morphframe:browser:01'};
  const imported=call('import_pbr_glb',args);check(JSON.stringify(call('import_pbr_glb',args))===JSON.stringify(imported),'retry identity');
  const state=call('inspect'),before=d.export_json();let staleRender=false;try{call('render_root_cpu',{revision:args.base_revision});}catch(e){staleRender=String(e).includes('stale_revision');}check(staleRender,'stale render');let staleBranch=false;try{call('branch',{branch:'old-base',base_revision:args.base_revision});}catch(e){staleBranch=String(e).includes('conflict');}check(staleBranch,'stale branch');const clip=imported.report.source_clips['0'];
  const project='morph-frames-'+name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS state roundtrip');
  call('branch',{branch:'clip',base_revision:state.revision});
  const request={revision:state.revision,clip,time:{numerator:1,denominator:2},shutter:{open:{numerator:-1,denominator:32},close:{numerator:1,denominator:32},samples:3}};
  const cpu=call('preview_at',{branch:'clip',request});const gpu=JSON.parse(await d.preview_frame_gpu('clip',JSON.stringify(request)));
  const a=cpu.passes.linear_rgb.flat(),b=gpu.passes.linear_rgb.flat();const rmse=Math.sqrt(a.reduce((s,x,i)=>s+(x-b[i])**2,0)/a.length);check(rmse<.002,'CPU/WebGPU parity');check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'nominal object IDs');
  const restoredCall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));restoredCall('branch',{branch:'clip',base_revision:state.revision});
  const again=JSON.parse(await restored.preview_frame_gpu('clip',JSON.stringify(request)));check(again.receipt.output_digest===gpu.receipt.output_digest,'restored render');
  const pending=d.preview_frame_gpu('clip',JSON.stringify(request));d.cancel_preview();let cancel=false;try{await pending;}catch(e){cancel=String(e).includes('cancel');}check(cancel,'GPU cancellation');
  check(d.export_json()===before,'immutable root');results.push({name,revision:state.revision,linear_rmse:rmse,object_mismatches:0,opfs_restored:true,staleRender,staleBranch,cancelled:cancel,cpu:cpu.passes,gpu:gpu.passes});restored.free();d.free();
 }
 for(const result of results.slice(1)){check(JSON.stringify(result.cpu.linear_rgb)===JSON.stringify(results[0].cpu.linear_rgb),'dense/sparse CPU pixels');check(JSON.stringify(result.gpu.linear_rgb)===JSON.stringify(results[0].gpu.linear_rgb),'dense/sparse GPU pixels');}
 const d=new m.BrowserAgent('00000000000000000000000000002710');const before=d.export_json();const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));let malformed=false;
 try{call('import_pbr_scene',{json:${JSON.stringify(bad)},buffers:[Array.from(atob(${JSON.stringify(bin)}),c=>c.charCodeAt(0))],images:[],policy:{allow_approximations:true},settings,base_revision:call('inspect').revision,idempotency_key:'morphframe:browser:bad:01'});}catch(e){malformed=String(e).includes('gltf_scene');}
 check(malformed&&d.export_json()===before,'malformed input atomic');d.free();return {status:'passed',profile:'gltf2-morph-frames-v1',results,malformed_atomic:malformed,dense_sparse_pixels_identical:true,seconds:(performance.now()-at)/1000};
 })()`);
 for(const result of report.results){for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+result.name+'-'+backend+'.json',JSON.stringify(result[backend])+'\n');delete result[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
