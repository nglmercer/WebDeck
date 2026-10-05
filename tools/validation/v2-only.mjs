import fs from 'node:fs';
import path from 'node:path';
const root=path.resolve(import.meta.dirname,'../..');
const rules=[
 ['text dispatcher',/\bparse_legacy\b|ParsedCommand|crate::app::buttons/],
 ['command delimiters',/<\|§\|>/],
 ['retired socket events',/message_from_socket|json_data|command_error/],
 ['generation-dependent authorization',/strict:\s*bool|legacy:\s*bool|["']legacy["']/],
 ['automatic v1 conversion',/config\.v1\.backup|fn migrate\(|parse_stringified_list|check_config_hyphen_case/],
 ['retired HTTP routes',/["'`]\/(?:send-data|save_config|COMPLETE_save_config|save_single_button|save_buttons_only|get_config|create_folder|api\/boot|usage|upload_file|upload_filepath|upload_folderpath)["'`]/],
 ['old asset protocol',/\*\*uploaded\/|(?:strip_prefix|starts_with)\(["']\.config\//],
];
const violations=[];
function scan(directory){if(!fs.existsSync(directory))return;for(const e of fs.readdirSync(directory,{withFileTypes:true})){const p=path.join(directory,e.name);if(e.isDirectory()){scan(p);continue;}if(!/\.(rs|ts|svelte|mjs|rhai)$/.test(e.name)||/\.(test|spec)\./.test(e.name)||p===import.meta.filename)continue;const content=fs.readFileSync(p,'utf8').split('#[cfg(test)]')[0];for(const [n,line]of content.split('\n').entries()){if(/^\s*(?:\/\/|\*|#)/.test(line))continue;for(const[name,pattern]of rules)if(pattern.test(line))violations.push({surface:name,file:path.relative(root,p),line:n+1});}}}
for(const d of ['src','frontend/src','frontend/e2e','tools/validation','examples/plugins'])scan(path.join(root,d));
const schema=JSON.parse(fs.readFileSync(path.join(root,'contracts/v2.schema.json'),'utf8'));
if(schema.$defs.CommandRequest.properties.message||!schema.$defs.CommandRequest.properties.command?.$ref)violations.push({surface:'untyped command request',file:'contracts/v2.schema.json'});
for(const retired of ['static/icons/icon_black.ico','static/css','static/img','frontend/demo','webdeck/colors.json','webdeck/translations/misc','docs/v2/HISTORICAL_PROPOSAL.md','docs/v2/IMPLEMENTATION_PLAN.md','docs/v2/evidence/v2-only','src/app','src/application','src/adapters','src/domain/transport.rs','frontend/src/views','frontend/src/framework','contracts/legacy-commands.json','webdeck/commands.json'])if(fs.existsSync(path.join(root,retired)))violations.push({surface:'deprecated artifact still present',file:retired});
const report={complete:violations.length===0,violations};
console.log(process.argv.includes('--json')?JSON.stringify(report,null,2):`V2-only migration guard: ${violations.length} violations`);process.exitCode=violations.length?1:0;
