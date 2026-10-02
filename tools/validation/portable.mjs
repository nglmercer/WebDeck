import {spawn,execFileSync} from 'node:child_process';
import {readFileSync,mkdtempSync,mkdirSync,rmSync,existsSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
const root=path.resolve(import.meta.dirname,'../..');const archive=path.resolve(process.argv[2]);
const sha256=createHash('sha256').update(readFileSync(archive)).digest('hex');
if(!readFileSync(archive+'.sha256','utf8').startsWith(sha256))throw Error('Checksum mismatch');
const temp=mkdtempSync(path.join(tmpdir(),'webdeck-portable-'));
execFileSync('python3',['-c',`import zipfile,sys,pathlib,stat
root=pathlib.Path(sys.argv[2])
with zipfile.ZipFile(sys.argv[1]) as z:
 for e in z.infolist():
  p=pathlib.PurePosixPath(e.filename)
  assert not p.is_absolute() and '..' not in p.parts and p.parts[0]=='WebDeck'
  mode=e.external_attr>>16
  assert not stat.S_ISLNK(mode)
  target=root.joinpath(*p.parts)
  if e.is_dir(): target.mkdir(parents=True,exist_ok=True)
  else:
   target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(z.read(e));target.chmod(0o755 if mode&0o111 else 0o644)`,archive,temp]);
const install=path.join(temp,'WebDeck'),data=path.join(temp,'data');mkdirSync(data);
const child=spawn(path.join(install,process.platform==='win32'?'WebDeck.exe':'WebDeck'),['--no-tray','--host','127.0.0.1','--port','59992'],{cwd:install,env:{...process.env,WEBDECK_CONFIG_DIR:data},stdio:'ignore'});
const wait=ms=>new Promise(r=>setTimeout(r,ms));let browser;
try{const base='http://127.0.0.1:59992';let ready=false;for(let i=0;i<400;i++){try{if((await fetch(base+'/api/v2/boot')).ok){ready=true;break;}}catch{}await wait(25);}if(!ready)throw Error('Artifact failed readiness');const boot=await(await fetch(base+'/api/v2/boot')).json();if(boot.api_version!==2||!boot.layout?.folders||!Number.isSafeInteger(boot.revision))throw Error('Invalid boot contract');for(const p of ['/','/static/icons/icon.ico','/api/v2/commands'])if(!(await fetch(base+p)).ok)throw Error('Missing '+p);
const require=createRequire(path.join(root,'frontend/package.json'));browser=await require('@playwright/test').chromium.launch({headless:true,executablePath:process.env.WEBDECK_CHROMIUM??(existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined),args:['--no-sandbox']});const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(base);await page.getByRole('heading',{name:'Home',exact:true}).waitFor();if(errors.length)throw Error(errors.join('\n'));const report={artifact:path.basename(archive),sha256,profile:path.basename(archive).includes('-dev-')?'development':'release',checks:['extracted outside repository','checksum','canonical boot','packaged assets','usable Chromium UI','no page errors','no native actions invoked']};mkdirSync(path.join(root,'docs/v2/evidence/rewrite'),{recursive:true});writeFileSync(path.join(root,'docs/v2/evidence/rewrite/portable.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
}finally{await browser?.close();child.kill('SIGINT');await Promise.race([new Promise(r=>child.once('exit',r)),wait(2000)]);if(child.exitCode===null){child.kill('SIGKILL');await new Promise(r=>child.once('exit',r));}rmSync(temp,{recursive:true,force:true});}
