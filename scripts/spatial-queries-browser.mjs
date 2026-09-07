// Spatial query client; archive JSON stays opaque until Rust parses high mesh IDs.
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8783';
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
 const native=JSON.parse(await fs.readFile(root+'/workflow_report.json','utf8'));
 const initial=await fs.readFile(root+'/initial-archive/document/document.json','utf8');
 const report=await evaluate(`(async()=>{
  const m=await import('/pkg/render_web.js');await m.default();const native=${JSON.stringify(native)};
  const check=(v,s)=>{if(!v)throw new Error(s);};
  let d=m.BrowserAgent.import_json(${JSON.stringify(initial)});const before=d.export_json();
  const call=(request,agent=d)=>JSON.parse(agent.dispatch(JSON.stringify({version:0,operation:{method:'query_geometry',request}})));
  const rows=[];
  for(const row of native.queries){const result=call(row.request);check(JSON.stringify(result.hits)===JSON.stringify(row.result.hits),'native/Wasm exact geometry results');rows.push(result);}
  check(JSON.stringify(rows.map(r=>r.cost.cache_reused))===JSON.stringify([false,true,true,true,true,false,true,false]),'warm reuse and one-index eviction across sources');
  check(d.export_json()===before,'queries preserve whole document');
  const invalid=[];const reject=(name,request,code)=>{let error;try{call(request);}catch(e){error=String(e);}check(error&&error.includes(code),name+' rejected: '+error);invalid.push({name,error});};
  const q=native.queries[0].request;
  reject('stale',{...q,base_revision:'sha256:'+'0'.repeat(64)},'stale_revision');
  reject('conditioned-frame',{...q,mesh:native.conditioned_mesh},'degenerate');
  reject('budget',{...q,budget:{...q.budget,max_item_tests:1}},'budget');
  reject('triangulation-budget',{...q,budget:{...q.budget,max_triangulation_work:1}},'budget');
  reject('result-budget',{...q,budget:{...q.budget,max_result_bytes:1}},'budget');
  reject('unknown',{...q,private_index:0},'unknown field');
  reject('metric',{...q,query:{kind:'nearest_surface',point:[0,0,0],max_distance_meters:-1}},'query_metric');
  check(d.export_json()===before&&call(q).cost.cache_reused,'failed query preserves document and cache');
  const generic=m.BrowserDocument.import_json(d.export_json());
  const receipt=JSON.parse(generic.execute(JSON.stringify(native.placement.request)));
  check(receipt.revision===native.revision,'ordinary placement native/Wasm revision');
  d.free();d=m.BrowserAgent.import_json(generic.export_json());generic.free();
  const placed=d.export_json();const updated={...q,base_revision:native.revision};
  check(JSON.stringify(call(updated).hits)===JSON.stringify(native.queries[0].result.hits),'asset-local query stable after marker placement');
  const name='spatial-queries-'+Date.now();await d.save(name,undefined,false);const restored=await m.BrowserAgent.load(name);
  check(restored.export_json()===placed,'OPFS restores exact opaque high-ID document');
  const recovered=call(updated,restored);check(!recovered.cost.cache_reused,'recovery rebuilds derived index');
  check(JSON.stringify(recovered.hits)===JSON.stringify(native.queries[0].result.hits),'recovered stable IDs');
  const dispatch=(agent,method,args={})=>JSON.parse(agent.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  dispatch(d,'branch',{branch:'spatial-render',base_revision:native.revision});
  const cpu=dispatch(d,'preview',{branch:'spatial-render',revision:native.revision});
  const gpu=JSON.parse(await d.preview_gpu('spatial-render',native.revision));
  dispatch(restored,'branch',{branch:'restored-render',base_revision:native.revision});
  const again=dispatch(restored,'preview',{branch:'restored-render',revision:native.revision});
  check(JSON.stringify(again.passes)===JSON.stringify(cpu.passes),'recovered CPU render exact');
  check(d.export_json()===placed,'render and queries retain document');
  const wasm=await(await fetch('/pkg/render_web_bg.wasm')).arrayBuffer();const package_sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',wasm)),x=>x.toString(16).padStart(2,'0')).join('');
  d.free();restored.free();return {status:'passed',package_sha256,queries:rows,invalid,opfs_restored:true,high_ids_preserved:true,revision:native.revision,cpu:cpu.passes,gpu:gpu.passes};
 })()`);
 if(report.package_sha256!==createHash('sha256').update(await fs.readFile('web/pkg/render_web_bg.wasm')).digest('hex'))throw new Error('served package mismatch');
 for(const kind of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+kind+'.json',JSON.stringify(report[kind])+'\n');delete report[kind];}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
