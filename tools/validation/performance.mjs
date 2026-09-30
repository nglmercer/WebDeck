// Reproducible safe workload: isolated configs, no real input/integrations.
// WEBDECK_BASELINE_ROOT/BINARY refer to a frozen checkout/build of the base.
import { spawn } from 'node:child_process';
import { mkdtempSync,readFileSync,writeFileSync,mkdirSync,rmSync,readdirSync } from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createRequire} from 'node:module';
import {gzipSync} from 'node:zlib';
const root=path.resolve('.');
const require=createRequire(path.join(root,'frontend/package.json'));
const {chromium}=require('@playwright/test');
const browser=await chromium.launch({headless:true,executablePath:process.env.WEBDECK_CHROMIUM ?? '/usr/bin/chromium',args:['--no-sandbox']});
const config=JSON.parse(readFileSync(path.join(root,'webdeck/config_default.json'),'utf8'));
Object.assign(config.settings,{show_popup:false,auto_updates:false,app_admin:false,automatic_firewall_bypass:false,language:'en_US',allowed_networks:['127.0.0.1/32'],dev_mode:false});
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function distribution(values){const sorted=[...values].sort((a,b)=>a-b);return {samples:sorted.length,median_ms:sorted[Math.floor(sorted.length/2)],p95_ms:sorted[Math.min(sorted.length-1,Math.floor(sorted.length*.95))]};}
async function measure(label,cwd,binary,port){
 const directory=mkdtempSync(path.join(tmpdir(),'webdeck-benchmark-'));writeFileSync(path.join(directory,'config.json'),JSON.stringify(config));
 for(const dir of ['themes','user_uploads','plugins'])mkdirSync(path.join(directory,dir));
 const started=performance.now();const child=spawn(binary,[`--host=127.0.0.1`,`--port=${port}`,'--no-tray','--no-admin','--no-auto-update','--force-start'],{cwd,env:{...process.env,WEBDECK_CONFIG_DIR:directory},stdio:'ignore'});
 const base=`http://127.0.0.1:${port}`;
 try{
  let ready=false;for(let i=0;i<400;i++){try{if((await fetch(base+'/api/boot')).ok){ready=true;break;}}catch{}await wait(25);}
  if(!ready)throw new Error(label+' failed readiness');const startup=performance.now()-started;
  const boot=[],command=[],ui=[];
  for(let i=0;i<35;i++){let now=performance.now();const r=await fetch(base+'/api/boot');if(!r.ok)throw new Error('boot failed');await r.json();if(i>=5)boot.push(performance.now()-now);}
  for(let i=0;i<35;i++){let now=performance.now();const r=await fetch(base+'/send-data',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({message:'/debug-send {}'})});if(!(await r.json()).success)throw new Error('debug failed');if(i>=5)command.push(performance.now()-now);}
  const page=await browser.newPage();
  for(let i=0;i<5;i++){const now=performance.now();await page.goto(base);await page.locator('form.form').first().waitFor({state:'visible'});ui.push(performance.now()-now);}
  await page.close();
  let rss_kib=null,idle_cpu_ticks=null;
  try{rss_kib=Number(readFileSync(`/proc/${child.pid}/status`,'utf8').match(/^VmRSS:\s+(\d+)/m)[1]);const ticks=()=>{const fields=readFileSync(`/proc/${child.pid}/stat`,'utf8').split(') ')[1].split(' ');return Number(fields[11])+Number(fields[12]);};const before=ticks();await wait(1000);idle_cpu_ticks=ticks()-before;}catch{}
  const assets=path.join(cwd,'frontend/dist/assets');const js=readdirSync(assets).filter(f=>f.endsWith('.js')).map(f=>readFileSync(path.join(assets,f)));
  return {label,startup_to_boot_ms:startup,boot:distribution(boot),command:distribution(command),usable_ui:distribution(ui),idle_rss_kib:rss_kib,idle_cpu_ticks_1s:idle_cpu_ticks,bundle_js_bytes:js.reduce((sum,v)=>sum+v.length,0),bundle_gzip_bytes:js.reduce((sum,v)=>sum+gzipSync(v).length,0)};
 }finally{child.kill('SIGINT');await Promise.race([new Promise(resolve=>child.once('exit',resolve)),wait(2000)]);if(child.exitCode===null)child.kill('SIGKILL');rmSync(directory,{recursive:true,force:true});}
}
try{
 const measurements=[];
 if(process.env.WEBDECK_BASELINE_ROOT&&process.env.WEBDECK_BASELINE_BINARY)measurements.push(await measure('baseline',process.env.WEBDECK_BASELINE_ROOT,process.env.WEBDECK_BASELINE_BINARY,59994));
 measurements.push(await measure('v2',root,path.join(root,'target/debug/webdeck'),59993));
 const report={date:new Date().toISOString(),node:process.version,platform:process.platform,arch:process.arch,profile:'debug; same isolated default config; native debug action only; cached build; sequential requests',measurements};
 writeFileSync('docs/v2/evidence/performance.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
}finally{await browser.close();}
