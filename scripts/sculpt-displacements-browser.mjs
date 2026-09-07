// Spatial query client; archive JSON stays opaque until Rust parses high mesh IDs.
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8785';
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
 const expected=await fs.readFile(root+'/archive/document/document.json','utf8');
 const report=await evaluate(`(async()=>{
  const m=await import('/pkg/render_web.js');await m.default();const native=${JSON.stringify(native)};
  const check=(v,s)=>{if(!v)throw new Error(s);};
  const d=m.BrowserAgent.import_json(${JSON.stringify(initial)});
  const call=(method,args={},agent=d)=>JSON.parse(agent.dispatch(JSON.stringify({version:0,operation:{method,...args}})));
  const before=d.export_json();const baseline=call('render_root_cpu',{revision:native.initial_revision});
  check(!baseline.sculpt_evaluations[0][1].basis_reused,'cold session basis');
  const q=native.edits[0].request;const invalid=[];
  const reject=(name,request,code)=>{let error;try{call('author_sculpt',{request});}catch(e){error=String(e);}check(error&&error.includes(code),name+' '+error);invalid.push({name,error});};
  reject('stale',{...q,base_revision:'sha256:'+'0'.repeat(64)},'stale_revision');
  reject('duplicate',{...q,points:[...q.points,...q.points]},'sculpt_point');
  reject('unknown-point',{...q,points:[{point:'1',delta_meters:[0,0,1]}]},'sculpt_point');
  reject('copy-budget',{...q,budget:{...q.budget,max_copy_bytes:1}},'budget');
  check(d.export_json()===before,'rejected operations preserve document');
  const edited=call('author_sculpt',{request:q});check(edited.receipt.revision===native.revision,'native/Wasm authored revision');
  check(JSON.stringify(edited.report)===JSON.stringify(native.edits[0].result.report),'native/Wasm authoring costs');
  call('author_sculpt',{request:q});
  const cpu=call('render_root_cpu',{revision:native.revision});
  check(cpu.sculpt_evaluations[0][1].basis_reused&&cpu.sculpt_evaluations[0][1].changed_points===2,'actual session local refit');
  check(JSON.stringify(cpu.passes.depth_meters)!==JSON.stringify(baseline.passes.depth_meters),'displaced surface visible');
  check(call('render_root_cpu',{revision:native.revision}).sculpt_evaluations[0][1].exact_geometry_reused,'exact repeat reuse');
  // Durable receipt metadata differs between adapters; snapshots must match.
  const expected=m.BrowserAgent.import_json(${JSON.stringify(expected)});
  check(JSON.stringify(JSON.parse(d.export_json()).snapshot)===JSON.stringify(JSON.parse(expected.export_json()).snapshot),'native/Wasm exact snapshot');expected.free();
  call('branch',{branch:'sculpt-gpu',base_revision:native.revision});
  const gpu=JSON.parse(await d.preview_gpu('sculpt-gpu',native.revision));
  const stored=d.export_json();const name='sculpt-displacements-'+Date.now();await d.save(name,undefined,false);const restored=await m.BrowserAgent.load(name);
  check(restored.export_json()===stored,'OPFS exact recovery');
  const cold=call('render_root_cpu',{revision:native.revision},restored);
  check(!cold.sculpt_evaluations[0][1].basis_reused,'OPFS cold reconstruction');
  check(JSON.stringify(cold.passes)===JSON.stringify(cpu.passes),'fresh/refit exact CPU passes');
  const undone=call('author_sculpt',{request:native.undo.request});check(undone.report.output===native.source,'clear returns empty asset');
  const cleared=call('render_root_cpu',{revision:undone.receipt.revision});
  check(JSON.stringify(cleared.passes)===JSON.stringify(baseline.passes),'clear returns original render');
  const wasm=await(await fetch('/pkg/render_web_bg.wasm')).arrayBuffer();const package_sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',wasm)),x=>x.toString(16).padStart(2,'0')).join('');
  d.free();restored.free();return {status:'passed',package_sha256,invalid,opfs_restored:true,high_ids_preserved:true,revision:native.revision,costs:{cold:baseline.sculpt_evaluations,warm:cpu.sculpt_evaluations,reopened:cold.sculpt_evaluations,cleared:cleared.sculpt_evaluations},cpu:cpu.passes,gpu:gpu.passes};
 })()`);
 if(report.package_sha256!==createHash('sha256').update(await fs.readFile('web/pkg/render_web_bg.wasm')).digest('hex'))throw new Error('served package mismatch');
 for(const kind of ['cpu','gpu']){await fs.writeFile(root+'/browser-'+kind+'.json',JSON.stringify(report[kind])+'\n');delete report[kind];}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
