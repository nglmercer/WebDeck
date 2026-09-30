// Test the actual ZIP outside the repository, with isolated user data.
import {spawn,execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {mkdtempSync,readFileSync,writeFileSync,mkdirSync,rmSync,statSync,existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createRequire} from 'node:module';
const root=path.resolve('.');
const archive=path.resolve(process.argv[2] ?? `dist/WebDeck-${process.platform==='win32'?'windows':process.platform}-${process.arch==='x64'?'x86_64':process.arch}-portable.zip`);
const expected=readFileSync(archive+'.sha256','utf8').split(/\s/)[0];
if(createHash('sha256').update(readFileSync(archive)).digest('hex')!==expected)throw new Error('Archive checksum mismatch');
const directory=mkdtempSync(path.join(tmpdir(),'webdeck-portable-'));
execFileSync('python3',['-c',`
import zipfile,sys,pathlib,stat
root=pathlib.Path(sys.argv[2])
with zipfile.ZipFile(sys.argv[1]) as z:
 for item in z.infolist():
  p=pathlib.PurePosixPath(item.filename)
  assert p.parts[0]=='WebDeck' and not p.is_absolute() and '..' not in p.parts
  mode=item.external_attr>>16
  assert not stat.S_ISLNK(mode)
  target=root.joinpath(*p.parts)
  if item.is_dir(): target.mkdir(parents=True,exist_ok=True)
  else:
   target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(z.read(item))
   target.chmod(0o755 if mode&0o111 else 0o644)
`,archive,directory]);
const installation=path.join(directory,'WebDeck');
const data=path.join(directory,'data');mkdirSync(data);
for(const name of ['themes','plugins','user_uploads'])mkdirSync(path.join(data,name));
const config=JSON.parse(readFileSync(path.join(installation,'webdeck/config_default.json'),'utf8'));
Object.assign(config.settings,{show_popup:false,auto_updates:false,automatic_firewall_bypass:false,app_admin:false,language:'en_US',allowed_networks:['127.0.0.1/32']});
writeFileSync(path.join(data,'config.json'),JSON.stringify(config));
const binary=path.join(installation,process.platform==='win32'?'WebDeck.exe':'WebDeck');
if(process.platform!=='win32' && !(statSync(binary).mode&0o111))throw new Error('Binary is not executable');
const child=spawn(binary,['--no-tray','--no-admin','--no-auto-update','--force-start','--host=127.0.0.1','--port=59992'],{cwd:installation,env:{...process.env,WEBDECK_CONFIG_DIR:data},stdio:'ignore'});
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let browser;
try{
 const base='http://127.0.0.1:59992';let ready=false;
 for(let i=0;i<400;i++){try{if((await fetch(base+'/api/boot')).ok){ready=true;break;}}catch{}await wait(25);}
 if(!ready)throw new Error('Portable server failed readiness');
 const boot=await (await fetch(base+'/api/boot')).json();
 if(!boot.config || !boot.config_revision)throw new Error('Missing boot contract');
 for(const route of ['/','/static/css/style.css','/api/v2/commands']){
  if(!(await fetch(base+route)).ok)throw new Error('Portable asset/route missing: '+route);
 }
 const require=createRequire(path.join(root,'frontend/package.json'));
 browser=await require('@playwright/test').chromium.launch({headless:true,executablePath:process.env.WEBDECK_CHROMIUM ?? (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined),args:['--no-sandbox']});
 const page=await browser.newPage();const errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(base);await page.locator('form.form').first().waitFor({state:'visible'});
 if(errors.length)throw new Error(errors.join('\n'));
 const report={date:new Date().toISOString(),artifact:path.basename(archive),sha256:expected,profile:'release; extracted outside repository; isolated data; no native commands',checks:['checksum','executable permissions','boot','static CSS','v2 registry','Chromium usable deck','no page errors']};
 writeFileSync(path.join(root,'docs/v2/evidence/portable.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
}finally{
 await browser?.close();child.kill('SIGINT');
 await Promise.race([new Promise(resolve=>child.once('exit',resolve)),wait(5000)]);
 if(child.exitCode===null){child.kill('SIGKILL');await new Promise(resolve=>child.once('exit',resolve));}
 rmSync(directory,{recursive:true,force:true});
}
