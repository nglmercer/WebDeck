// The development harness uses Node; the product process has no JS tools in PATH.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '../..');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-runtime-'));
const emptyPath = path.join(directory, 'empty-path');
mkdirSync(emptyPath);
const plugin = path.join(directory, 'plugins', 'example'); mkdirSync(plugin, {recursive:true});
const source = 'export function invoke_action(action,args,ctx){ return {text:args.text}; }';
writeFileSync(path.join(plugin, 'index.js'), source);
writeFileSync(path.join(plugin, 'webdeck.json'), JSON.stringify({schema_version:2,id:'example',version:'2.0.0',entry:'index.js',backend:'sandbox_js',digest:createHash('sha256').update(source).digest('hex'),origin:'local',contract:'',actions:[{id:'echo',label:'Echo',capabilities:['read'],arguments:{text:{type:'string',required:true}},result:{type:'object',required:true}}]}));
const binary = process.env.WEBDECK_RUNTIME_BINARY ?? path.join(root, 'target', 'debug', process.platform === 'win32' ? 'webdeck.exe' : 'webdeck');
const child = spawn(binary, ['--no-tray','--host','127.0.0.1','--port','59991'], {cwd:root,env:{...process.env, PATH:emptyPath, WEBDECK_CONFIG_DIR:directory},stdio:['ignore','ignore','pipe']});
let failure; let stderr=''; child.on('error',error=>failure=error); child.stderr.on('data',bytes=>stderr+=bytes);
const exited = new Promise(resolve=>child.once('exit',resolve));
const base='http://127.0.0.1:59991/api/v2/';
try {
  const deadline=Date.now()+10000;
  for (;;) {
    if (failure) throw failure;
    if (child.exitCode !== null) throw new Error('Runtime exited before readiness');
    try { if ((await fetch(base+'boot')).ok) break; } catch {}
    if (Date.now()>deadline) throw new Error('Runtime readiness timed out');
    await new Promise(resolve=>setTimeout(resolve,25));
  }
  let requestNumber=0;
  async function invoke(command) {
    const response=await fetch(base+'commands',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({request_id:'runtime-'+(++requestNumber),command})});
    assert.equal(response.status,200); const event=await response.json(); assert.equal(event.state,'completed'); return event.result;
  }
  assert.deepEqual(await invoke({type:'debug',data:{text:'embedded'}}),{data:{text:'embedded'}});
  assert.deepEqual(await invoke({type:'script',language:'javascript',source:{type:'inline',code:"ctx.invoke({type:'debug',data:{text:'script'}})"}}),{value:{data:{text:'script'}}});
  assert.deepEqual(await invoke({type:'plugin',plugin_id:'example',version:'2.0.0',action_id:'echo',args:{text:'plugin'}}),{plugin_id:'example',version:'2.0.0',action_id:'echo',value:{text:'plugin'}});
  assert.deepEqual(await invoke({type:'workflow',workflow:{type:'sequence',steps:[{type:'delay',milliseconds:1},{type:'command',command:{type:'debug',data:{text:'workflow'}}}]}}),[null,{data:{text:'workflow'}}]);
  const status=await (await fetch(base+'runtime')).json(); assert.equal(status.runtime,'napi-vm'); assert.equal(status.healthy,true); assert(status.plugins.some(p=>p.id==='example'));
  const catalog=await (await fetch(base+'commands')).json(); assert(catalog.commands.some(c=>c.id==='workflow')); assert(catalog.plugins.some(p=>p.id==='builtin.obs'));
  assert.equal((await fetch(base+'runtime/reload',{method:'POST'})).status,200);
  console.log('Embedded runtime smoke passed with Node, Bun and npm absent from product PATH.');
} finally {
  child.kill('SIGINT'); const timer=setTimeout(()=>child.kill('SIGKILL'),5000);
  await exited; clearTimeout(timer); rmSync(directory,{recursive:true,force:true});
  assert(!stderr.includes('VM probe'), 'Guest errors must not be printed to diagnostics');
}
