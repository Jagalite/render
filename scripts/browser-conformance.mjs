// External test harness only. No application behavior is implemented here.
import fs from 'node:fs/promises';
const port = process.env.RENDER_CDP_PORT || '9223';
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = targets.find(x => x.type === 'page');
if (!page) throw new Error('No isolated test page available');
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {socket.onopen = resolve; socket.onerror = reject;});
let sequence = 0;
const pending = new Map();
socket.onmessage = event => {const message = JSON.parse(event.data); if (message.id) {const request = pending.get(message.id); pending.delete(message.id); if (message.error) request.reject(message.error); else request.resolve(message.result);}};
function command(method, params = {}) {const id = ++sequence; return new Promise((resolve,reject) => {pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});}
await command('Page.enable');
await command('Runtime.enable');
await command('Page.navigate',{url:'http://127.0.0.1:8765/'});
const result = await command('Runtime.evaluate', {expression: `new Promise((resolve,reject)=>{const deadline=Date.now()+120000;const poll=()=>{const e=document.getElementById('report');if(e&&e.dataset.status==='passed')resolve(e.textContent);else if(e&&e.dataset.status==='failed')reject(new Error(e.textContent));else if(Date.now()>deadline)reject(new Error('Browser conformance timed out'));else setTimeout(poll,250);};poll();})`,awaitPromise:true,returnByValue:true});
if (result.exceptionDetails) {socket.close();throw new Error(JSON.stringify(result.exceptionDetails));}
const report = JSON.parse(result.result.value);
async function evaluate(expression) {
    const response = await command('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});
    if(response.exceptionDetails)throw new Error(JSON.stringify(response.exceptionDetails));
    return response.result.value;
}
report.external_browser_api = await evaluate(`(async()=>{
    const m=await import('/pkg/render_web.js');
    const d=new m.BrowserDocument('00000000000000000000000000000077');
    const request={version:0,base_revision:d.revision(),idempotency_key:'browser:external:01',max_added_bytes:10000,commands:[{operation:'create_entity',entity:{id:'00000000000000000000000000000001',name:'External client',parent:null,mesh:null,material:null,transform:{columns:[[1,0,0],[0,1,0],[0,0,1],[0,0,0]],operations:[]}}}]};
    const candidate=d.prepare(JSON.stringify(request));const receipt=JSON.parse(d.inspect_candidate(candidate));
    if(receipt.base_revision!==request.base_revision)throw new Error('candidate base');
    d.commit(candidate);const fork=d.branch();if(fork.revision()!==d.revision())throw new Error('branch');
    const settings={width:4,height:4,samples:1,seed:42,max_depth:1,max_bytes:1048576,environment:[0.1,0.2,0.3],light:{position:[0,4,0],intensity:[1,1,1]},camera:{position:[0,0,3],target:[0,0,0],up:[0,1,0],vertical_fov_radians:0.7}};
    const image=JSON.parse(await m.render_document(d.export_json(),JSON.stringify(settings)));
    if(image.linear_rgb.length!==16||image.linear_rgb.some(p=>p.some((x,i)=>Math.abs(x-settings.environment[i])>1e-6)))throw new Error('analytic environment');
    fork.free();d.free();return true;
})()`);
await command('Storage.overrideQuotaForOrigin',{origin:'http://127.0.0.1:8765',quotaSize:1});
try {report.storage_quota = JSON.parse(await evaluate(`(async()=>{const m=await import('/pkg/render_web.js');return await m.storage_quota_probe();})()`));}
finally {await command('Storage.overrideQuotaForOrigin',{origin:'http://127.0.0.1:8765'});}
const previousRevision = await evaluate(`(async()=>{const m=await import('/pkg/render_web.js');return await m.stage_interrupted_write();})()`);
await command('Page.reload',{ignoreCache:true});
await new Promise(resolve=>setTimeout(resolve,2000));
const recoveredRevision = await evaluate(`(async()=>{const m=await import('/pkg/render_web.js');return await m.recover_interrupted_write();})()`);
if(previousRevision!==recoveredRevision)throw new Error('Tab interruption changed committed root');
report.tab_reload_preserves_unclosed_root = true;
report.browser = await command('Browser.getVersion');
await fs.mkdir('artifacts/evidence',{recursive:true});
await fs.writeFile('artifacts/evidence/browser_conformance.json',JSON.stringify(report,null,2)+'\n');
const shot = await command('Page.captureScreenshot',{format:'png',captureBeyondViewport:true});
await fs.writeFile('artifacts/evidence/browser.png',Buffer.from(shot.data,'base64'));
console.log(JSON.stringify(report));
socket.close();
