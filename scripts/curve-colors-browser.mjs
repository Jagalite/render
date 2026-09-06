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
 for(const row of native.variants)cases.push({...row,bytes:row.kind==='hair'?[...await fs.readFile(row.source.path)]:null,archive:await fs.readFile(row.archive,'utf8'),nativeGlb:[...await fs.readFile(row.glb)]});
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const at=performance.now();const results=[];
 const dispatch=d=>(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 for(const row of ${JSON.stringify(cases)}){
  const d=row.kind==='hair'?new m.BrowserAgent('${(10400).toString(16).padStart(32,'0')}'):m.BrowserAgent.import_json(row.archive);const call=dispatch(d);let imported=null;
  if(row.kind==='hair'){
   const empty=call('inspect');check(empty.revision===row.empty_revision,'same empty revision');
   const request={bytes:row.bytes,policy:row.source.policy,settings:row.settings,base_revision:empty.revision,idempotency_key:'curve:rgba:import:001'};
   imported=call('import_hair',request);check(JSON.stringify({...imported.report,observed_derived_json_bytes:undefined})===JSON.stringify({...row.report,observed_derived_json_bytes:undefined}),'same typed import report');call('import_hair',request);
   let stale=false;try{call('import_hair',{...request,idempotency_key:'curve:rgba:stale:001'});}catch(e){stale=String(e).includes('import_target');}check(stale,'nonempty import rejected');
  }
  check(call('inspect').revision===row.revision,'same native revision');const evaluated=call('render_root_cpu',{revision:row.revision});
  if(imported)check(evaluated.conversions.reduce((sum,c)=>sum+c.derived_bytes,0)===imported.report.observed_derived_json_bytes,'observed derived bytes');
  const before=d.export_json();call('branch',{branch:'color',base_revision:row.revision});
  const cpu=call('preview',{branch:'color',revision:row.revision});check(JSON.stringify(cpu.passes)===JSON.stringify(evaluated.passes),'root/branch agreement');const gpu=JSON.parse(await d.preview_gpu('color',row.revision));
  const compare=(a,b)=>{const x=a.linear_rgb.flat(),y=b.linear_rgb.flat();check(x.length===y.length,'dimensions');const error=Math.sqrt(x.reduce((sum,v,i)=>sum+(v-y[i])**2,0)/x.length);check(error<.002,'color parity '+error);return error;};compare(cpu.passes,gpu.passes);check(JSON.stringify(cpu.passes.object_ids)===JSON.stringify(gpu.passes.object_ids),'objects');
  const pending=d.preview_gpu('color',row.revision);d.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'GPU cancellation');
  const request={revision:row.revision,policy:row.export_policy};const exported=call('export_glb',{request});check(JSON.stringify(exported.glb)===JSON.stringify(row.nativeGlb),'same native/browser GLB '+row.name);check(JSON.stringify(call('export_glb',{request}))===JSON.stringify(exported),'deterministic export');
  let budget=false;try{call('export_glb',{request:{...request,policy:{...row.export_policy,max_bytes:64}}});}catch(e){budget=String(e).includes('budget');}check(budget,'export budget');
  const round=new m.BrowserAgent('${(10400).toString(16).padStart(32,'0')}');const rc=dispatch(round);const imp=rc('import_pbr_glb',{bytes:exported.glb,policy:{allow_approximations:true},settings:row.settings,base_revision:rc('inspect').revision,idempotency_key:'curve:rgba:round:001'});const state=rc('inspect');rc('branch',{branch:'round',base_revision:state.revision});
  const mapping={};for(const [old,node] of Object.entries(exported.report.source_entities)){const parent=imp.report.source_nodes[node];const leaves=state.snapshot.entities.filter(e=>e.parent===parent&&e.mesh!==null);check(leaves.length===1,'node mapping');mapping[old]=leaves[0].id;}
  const roundtrip=[];for(const backend of ['cpu','gpu']){const rendered=backend==='cpu'?rc('preview',{branch:'round',revision:state.revision}):JSON.parse(await round.preview_gpu('round',state.revision));const error=compare(cpu.passes,rendered.passes);check(JSON.stringify(cpu.passes.object_ids.map(x=>x===null?null:mapping[x]))===JSON.stringify(rendered.passes.object_ids),'roundtrip objects');roundtrip.push({backend,linear_rmse:error});}
  const project='curve-colors-'+row.name+'-'+Date.now();await d.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===before,'OPFS recovery');const loaded=dispatch(restored);loaded('branch',{branch:'color',base_revision:row.revision});check(JSON.stringify(loaded('preview',{branch:'color',revision:row.revision}).passes)===JSON.stringify(cpu.passes),'recovered pixels');check(d.export_json()===before,'immutable root');
  results.push({name:row.name,opfs_restored:true,native_browser_glb_identical:true,roundtrip,cpu:cpu.passes,gpu:gpu.passes,report:imported?.report??null});restored.free();round.free();d.free();
 }
 return {status:'passed',results,seconds:(performance.now()-at)/1000};
 })()`);
 for(const row of report.results){for(const backend of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+row.name+'-'+backend+'.json',JSON.stringify(row[backend])+'\n');delete row[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
