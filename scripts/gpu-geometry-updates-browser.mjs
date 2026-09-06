// GPU cache conformance client; all rendering and cache decisions remain Rust.
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
const root=process.argv[2];
const origin=process.env.RENDER_TEST_ORIGIN??'http://127.0.0.1:8782';
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
 const native=JSON.parse(await fs.readFile(root+'/native_report.json','utf8'));
 const request={version:0,cases:[]};
 for(const row of native.cases)request.cases.push({name:row.name,revision:row.revision,document:JSON.parse(await fs.readFile(root+'/'+row.name+'/document.json','utf8'))});
 const report=await evaluate(`(async()=>{
  const m=await import('/pkg/render_web.js');await m.default();const request=${JSON.stringify(request)};
  const invalid=[];const reject=async(name,value,code)=>{let error;try{await m.gpu_geometry_update_conformance(JSON.stringify(value));}catch(e){error=String(e);}if(!error||(code&&!error.includes(code)))throw new Error(name+' did not reject: '+error);invalid.push({name,error});};
  await reject('version',{...request,version:1},'request');await reject('empty',{version:0,cases:[]},'request');
  await reject('duplicate',{version:0,cases:[request.cases[0],request.cases[0]]},'request');
  await reject('stale',{version:0,cases:[{...request.cases[0],revision:'sha256:'+'0'.repeat(64)}]},'stale_revision');
  await reject('unknown',{...request,extra:true},'unknown field');
  const over=structuredClone(request);over.cases[0].document.snapshot.render_settings.samples=9;
  // Change the pinned revision only for the explicit profile-limit test by using
  // the ordinary validated document adapter to derive its new snapshot identity.
  const imported=m.BrowserDocument.import_json(JSON.stringify(over.cases[0].document));over.cases[0].revision=imported.revision();imported.free();
  await reject('profile-budget',over,'budget');
  const report=JSON.parse(await m.gpu_geometry_update_conformance(JSON.stringify(request)));report.invalid=invalid;
  const wasm=await(await fetch('/pkg/render_web_bg.wasm')).arrayBuffer();report.package_sha256=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',wasm)),x=>x.toString(16).padStart(2,'0')).join('');return report;
 })()`);
 if(report.package_sha256!==createHash('sha256').update(await fs.readFile('web/pkg/render_web_bg.wasm')).digest('hex'))throw new Error('served package mismatch');
 for(const row of report.cases){for(const kind of ['cpu','gpu']){await fs.writeFile(root+'/'+row.name+'/browser-'+kind+'.json',JSON.stringify(row[kind])+'\n');delete row[kind];}}
 report.browser=await command('Browser.getVersion');await fs.writeFile(root+'/browser_report.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {socket.close();}
