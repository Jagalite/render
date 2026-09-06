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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));const variants=[];
 for(const name of ['mask','blend','opaque'])variants.push({name,asset:(await fs.readFile('fixtures/alpha-gltf/data/'+name+'.glb')).toString('base64')});
 const bad=JSON.parse(await fs.readFile(root+'/malformed.gltf','utf8'));const bin=(await fs.readFile('fixtures/alpha-gltf/data/mask.bin')).toString('base64');
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const settings=${JSON.stringify(native.settings)};const at=performance.now();const results=[];
 for(const {name,asset} of ${JSON.stringify(variants)}){
  const d=new m.BrowserAgent('00000000000000000000000000002648');const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  const args={bytes:Array.from(atob(asset),c=>c.charCodeAt(0)),policy:{allow_approximations:true},settings,base_revision:call('inspect').revision,idempotency_key:'alpha:browser:0001'};
  const imported=call('import_pbr_glb',args);check(JSON.stringify(call('import_pbr_glb',args))===JSON.stringify(imported),'retry identity');
  const state=call('inspect'),before=d.export_json();let stale=false;
  try{call('import_pbr_glb',{...args,idempotency_key:'alpha:stale:00001'});}catch(e){stale=String(e).includes('stale_revision');}check(stale,'stale import rejected');
  const project='alpha-'+name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS state roundtrip');
  call('branch',{branch:'alpha',base_revision:state.revision});
  const cpu=call('preview',{branch:'alpha',revision:state.revision});let gpuUnsupported=false;
  try{await d.preview_gpu('alpha',state.revision);}catch(e){gpuUnsupported=String(e).includes('unsupported_profile');}check(gpuUnsupported===(name!=='opaque'),'truthful GPU alpha support');
  const restoredCall=(method,args={})=>JSON.parse(restored.dispatch(JSON.stringify({version:0,operation:{method,...args}})));restoredCall('branch',{branch:'alpha',base_revision:state.revision});
  const again=restoredCall('preview',{branch:'alpha',revision:state.revision});check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'restored render');
  check(d.export_json()===before,'immutable root');results.push({name,revision:state.revision,opfs_restored:true,stale_rejected:true,gpu_unsupported:gpuUnsupported,cpu:cpu.passes});restored.free();d.free();
 }
 const d=new m.BrowserAgent('00000000000000000000000000002648');const before=d.export_json();const call=(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));let malformed=false;
 try{call('import_pbr_scene',{json:${JSON.stringify(bad)},buffers:[Array.from(atob(${JSON.stringify(bin)}),c=>c.charCodeAt(0))],images:[],policy:{allow_approximations:true},settings,base_revision:call('inspect').revision,idempotency_key:'alpha:browser:bad:01'});}catch(e){malformed=String(e).includes('gltf_scene');}
 check(malformed&&d.export_json()===before,'malformed input atomic');d.free();return {status:'passed',profile:'gltf2-alpha-cpu-v1',results,malformed_atomic:malformed,seconds:(performance.now()-at)/1000};
 })()`);
 for(const result of report.results){await fs.writeFile(root+'/browser-'+result.name+'-cpu.json',JSON.stringify(result.cpu)+'\n');delete result.cpu;}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
