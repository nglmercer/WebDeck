// Real HTTP/Socket.IO acceptance server with isolated data and fake effects.
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
const root = path.resolve('..');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-acceptance-'));
const config = JSON.parse(readFileSync(path.join(root,'webdeck/config_default.json'),'utf8'));
Object.assign(config.settings, { show_popup:false, auto_updates:false, automatic_firewall_bypass:false, app_admin:false, language:'en_US', dev_mode:true, allowed_networks:['127.0.0.1/32'], data_transfer_method:'http' });
Object.assign(config.front, { width:3, height:2, computer_usage_reload_time:60000, themes:[], buttons:{ index:[{name:'Work folder',message:'/folder work',image:'folder.png'},{name:'Debug action',message:'/debug-send {}',image:'folder.png'},{name:'Settings',message:'/open-config',image:'settings.png'},{name:'Reload deck',message:'/reload',image:'reload.png'},{VOID:'VOID'},{VOID:'VOID'}],work:[{name:'Home folder',message:'/folder index',image:'folder.png'},{VOID:'VOID'},{VOID:'VOID'},{VOID:'VOID'},{VOID:'VOID'},{VOID:'VOID'}] } });
writeFileSync(path.join(directory,'config.json'), JSON.stringify(config));
for (const name of ['themes','user_uploads','plugins']) mkdirSync(path.join(directory,name));
writeFileSync(path.join(directory,'themes','acceptance.css'), '/* title: Acceptance theme */\n.buttons-center{--acceptance-theme:1}');
const executable = process.env.WEBDECK_ACCEPTANCE_BINARY ?? path.join(root,'target/debug/webdeck' + (process.platform === 'win32' ? '.exe' : ''));
const child = spawn(executable,['--no-tray','--no-auto-update','--no-admin','--force-start','--host','127.0.0.1','--port','59996'], {cwd:root,env:{...process.env,WEBDECK_CONFIG_DIR:directory,WEBDECK_FAKE_EFFECTS:'1'},stdio:'inherit'});
let stopping = false;
function stop() { if (stopping) return; stopping = true; child.kill('SIGINT'); setTimeout(() => child.kill('SIGKILL'),5000).unref(); }
process.on('SIGTERM',stop); process.on('SIGINT',stop);
child.on('error',error => { console.error(error); rmSync(directory,{recursive:true,force:true}); process.exit(1); });
child.on('exit',code => { rmSync(directory,{recursive:true,force:true}); process.exit(stopping ? 0 : code ?? 1); });
