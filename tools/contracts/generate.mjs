import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const schema = JSON.parse(fs.readFileSync(path.join(root, 'contracts/v2.schema.json'), 'utf8'));
const defs = schema.$defs;
function ts(t) {
  if (t.$ref) return t.$ref.split('/').pop();
  if (t.const !== undefined) return JSON.stringify(t.const);
  if (t.enum) return t.enum.map(JSON.stringify).join(' | ');
  if (t.type === 'array') return `Array<${ts(t.items)}>`;
  return ({ string:'string', integer:'number', boolean:'boolean', object:'Record<string, unknown>' })[t.type] ?? 'unknown';
}
let types = '// Generated from contracts/v2.schema.json; run node tools/contracts/generate.mjs.\n';
for (const [name, t] of Object.entries(defs)) {
  if (!t.properties) types += `export type ${name} = ${ts(t)};\n`;
  else types += `export interface ${name} {\n${Object.entries(t.properties).map(([key,v]) => `  ${key}${t.required.includes(key)?'':'?'}: ${ts(v)};`).join('\n')}\n}\n`;
}
types += 'export type CommandEvent = CommandAccepted | CommandCompleted | CommandFailed;\n';
let rust = '// Generated from contracts/v2.schema.json; run node tools/contracts/generate.mjs.\nuse crate::domain::command::Capability;\nuse serde::Deserialize;\nuse serde_json::Value;\n';
function rs(t) {
 if (t.$ref) return t.$ref.split('/').pop();
 if (t.type === 'array') return `Vec<${rs(t.items)}>`;
 return ({string:'String',integer:'u64',boolean:'bool'})[t.type] ?? 'Value';
}
for (const name of ['CommandRequest','ConfigRequest','DeviceRequest']) {
 const t = defs[name];
 rust += `\n#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct ${name} {\n`;
 for (const [key,v] of Object.entries(t.properties)) rust += `    pub ${key}: ${t.required.includes(key)?rs(v):`Option<${rs(v)}>`},\n`;
 rust += '}\n';
}
for (const [file, contents] of [['frontend/src/contracts/v2.ts',types],['src/domain/transport.rs',rust]]) {
 const target = path.join(root,file);
 if (process.argv.includes('--check')) {
   if (!fs.existsSync(target) || fs.readFileSync(target,'utf8') !== contents) throw new Error(`Contract drift: ${file}`);
 } else fs.writeFileSync(target, contents);
}
