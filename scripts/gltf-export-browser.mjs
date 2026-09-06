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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));const recipe=JSON.parse(await fs.readFile('fixtures/gltf-export/cases.json','utf8'));
 const cases=[];for(const row of native.variants)cases.push({...row,asset:(await fs.readFile(row.source)).toString('base64'),nativeGlb:(await fs.readFile(root+'/'+row.name+'-export/scene/scene.glb')).toString('base64')});
 const report=await evaluate(`(async()=>{
 const m=await import('/pkg/render_web.js');const check=(v,s)=>{if(!v)throw new Error(s);};const settings=${JSON.stringify(native.settings)};const policy=${JSON.stringify(recipe.policy)};const at=performance.now();const results=[];
 const dispatch=d=>(method,args={})=>JSON.parse(d.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
 const bytes=s=>Array.from(atob(s),c=>c.charCodeAt(0));
 for(const row of ${JSON.stringify(cases)}){
  const {name,asset,nativeGlb}=row;const d=new m.BrowserAgent(${JSON.stringify(recipe.document_id)});const call=dispatch(d);const empty=call('inspect');
  call('import_pbr_glb',{bytes:bytes(asset),policy:{allow_approximations:true},settings,base_revision:empty.revision,idempotency_key:'gltfexport:browser:01'});
  const state=call('inspect'),before=d.export_json();check(state.revision===row.revision,'same native source revision');
  const request={revision:state.revision,at:row.at,policy};const exported=call('export_glb',{request});check(JSON.stringify(exported.glb)===JSON.stringify(bytes(nativeGlb)),'native/browser GLB bytes');
  check(JSON.stringify(call('export_glb',{request}))===JSON.stringify(exported),'deterministic export');
  for(const [reason,q] of [['stale_revision',{...request,revision:empty.revision}],['budget',{...request,policy:{...policy,max_bytes:64}}],['unsupported_gltf',{...request,policy:{...policy,allow_approximations:false}}]]){
   let rejected=false;try{call('export_glb',{request:q});}catch(e){rejected=String(e).includes(reason);}check(rejected,'export admission '+reason);
  }
  const r=new m.BrowserAgent(${JSON.stringify(recipe.document_id)});const round=dispatch(r);
  const imported=round('import_pbr_glb',{bytes:exported.glb,policy:{allow_approximations:true},settings,base_revision:round('inspect').revision,idempotency_key:'gltfexport:round:0001'});const rs=round('inspect');
  check(rs.revision===row.round_revision,'same roundtrip native revision');const mapping={};
  for(const [old,node] of Object.entries(exported.report.source_entities)){const parent=imported.report.source_nodes[node];const leaves=rs.snapshot.entities.filter(e=>e.parent===parent&&e.mesh!==null);check(leaves.length===1,'source node mapping');mapping[old]=leaves[0].id;}
  call('branch',{branch:'source',base_revision:state.revision});round('branch',{branch:'round',base_revision:rs.revision});
  const zero={numerator:0,denominator:1};const frame=row.at?{...row.at,revision:state.revision,shutter:{open:zero,close:zero,samples:1}}:null;
  const cpu=frame?call('preview_at',{branch:'source',request:frame}):call('preview',{branch:'source',revision:state.revision});const roundCpu=round('preview',{branch:'round',revision:rs.revision});
  const compare=(a,b)=>{const x=a.linear_rgb.flat(),y=b.linear_rgb.flat();check(x.length===y.length,'pixel dimensions');const rmse=Math.sqrt(x.reduce((s,v,i)=>s+(v-y[i])**2,0)/x.length);check(rmse<.00002,'roundtrip color error '+rmse);check(JSON.stringify(a.object_ids.map(i=>i===null?null:mapping[i]))===JSON.stringify(b.object_ids),'roundtrip IDs');check(a.depth_meters.every((v,i)=>Math.abs(v-b.depth_meters[i])<.00002),'roundtrip depth');check(a.normals_world.flat().every((v,i)=>Math.abs(v-b.normals_world.flat()[i])<.00002),'roundtrip normals');return rmse;};
  const comparisons=[{backend:'cpu',rmse:compare(cpu.passes,roundCpu.passes)}];let roundGpu=null;
  if(row.gpu){const gpu=JSON.parse(frame?await d.preview_frame_gpu('source',JSON.stringify(frame)):await d.preview_gpu('source',state.revision));roundGpu=JSON.parse(await r.preview_gpu('round',rs.revision));comparisons.push({backend:'gpu',rmse:compare(gpu.passes,roundGpu.passes)});const pending=r.preview_gpu('round',rs.revision);r.cancel_preview();let cancelled=false;try{await pending;}catch(e){cancelled=String(e).includes('cancel');}check(cancelled,'roundtrip GPU cancellation');}
  else{let rejected=false;try{await r.preview_gpu('round',rs.revision);}catch(e){rejected=String(e).includes('unsupported_profile');}check(rejected,'alpha GPU remains explicitly unsupported');}
  const project='gltf-export-'+name+'-'+Date.now();const archived=r.export_json();await r.save(project,undefined,false);const restored=await m.BrowserAgent.load(project);check(restored.export_json()===archived,'OPFS roundtrip');const loaded=dispatch(restored);loaded('branch',{branch:'round',base_revision:rs.revision});const again=loaded('preview',{branch:'round',revision:rs.revision});check(JSON.stringify(again.passes)===JSON.stringify(roundCpu.passes),'OPFS restored pixels');
  check(d.export_json()===before&&r.export_json()===archived,'immutable source and imported root');results.push({name,native_browser_glb_identical:true,opfs_restored:true,comparisons,glb_digest:exported.report.output_digest,cpu:roundCpu.passes,gpu:roundGpu?.passes??null});restored.free();r.free();d.free();
 }
 return {status:'passed',profile:'gltf2-evaluated-pbr-export-v1',results,seconds:(performance.now()-at)/1000};
 })()`);
 for(const result of report.results){for(const backend of ['cpu','gpu']){if(result[backend])await fs.writeFile(root+'/browser-'+result.name+'-'+backend+'.json',JSON.stringify(result[backend])+'\n');delete result[backend];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
