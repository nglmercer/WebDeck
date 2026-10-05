import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const defs = JSON.parse(fs.readFileSync(path.join(root, 'contracts/v2.schema.json'), 'utf8')).$defs;
const title = s => s.split(/[_.-]/).map(x => x[0].toUpperCase()+x.slice(1)).join('');
function ts(s) {
 if(s.$ref) return s.$ref.split('/').pop();
 if(s.oneOf) return s.oneOf.map(ts).join(' | ');
 if(s.const!==undefined) return JSON.stringify(s.const);
 if(s.enum) return s.enum.map(JSON.stringify).join(' | ');
 if(s.type==='array') return `Array<${ts(s.items)}>`;
 if(s.type==='object') return s.properties ? `{ ${Object.entries(s.properties).map(([k,v])=>`${k}${s.required?.includes(k) ? "" : "?"}: ${ts(v)}`).join('; ')} }` : `Record<string, ${s.additionalProperties && typeof s.additionalProperties === 'object' ? ts(s.additionalProperties) : 'unknown'}>`;
 return {null:'null',string:'string',integer:'number',number:'number',boolean:'boolean'}[s.type] ?? 'unknown';
}
function rs(s) {
 if(s.$ref) { const name=s.$ref.split('/').pop(); return name === 'WorkflowNode' ? `Box<${name}>` : name; }
 if(s.type==='array') return `Vec<${rs(s.items)}>`;
 if(s.type==='object') return `BTreeMap<String, ${rs(s.additionalProperties??{})}>`;
 if(s.const!==undefined) return typeof s.const==='number'?'u64': 'String';
 return {string:'String',integer:s.minimum<0?'i64':'u64',number:'f64',boolean:'bool'}[s.type] ?? 'Value';
}
const derives = '#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]';
let rust='// Generated. Edit contracts/v2.schema.json and run the generator.\nuse serde::{Deserialize, Serialize};\nuse serde_json::Value;\nuse std::collections::BTreeMap;\n';
let types='// Generated. Edit contracts/v2.schema.json and run the generator.\n';
for(const [n,s] of Object.entries(defs)) {
 types+=`export type ${n} = ${ts(s)};\n`;
 if(s.enum) rust+=`${derives}\n#[derive(Eq, Hash, Copy)]\npub enum ${n} {\n${s.enum.map(v=>` #[serde(rename = ${JSON.stringify(v)})] ${title(v)},`).join('\n')}\n}\n`;
 else if(s.oneOf) {
  if(s.oneOf.every(x=>x.$ref)) rust+=`${derives}\n#[serde(untagged)]\npub enum ${n} {\n${s.oneOf.map(x=>{let t=rs(x);return ` ${t}(${t}),`;}).join('\n')}\n}\n`;
  else rust+=`${derives}\n#[serde(tag = "type", deny_unknown_fields)]\npub enum ${n} {\n${s.oneOf.map(x=>{const v=x.properties.type.const; const f=Object.entries(x.properties).filter(([k])=>k!=='type');return ` #[serde(rename = ${JSON.stringify(v)})] ${title(v)}${f.length?` { ${f.map(([k,t])=>`${!x.required?.includes(k) ? '#[serde(default, skip_serializing_if = "Option::is_none")] ' : ''}${k === "type" ? "r#type" : k}: ${x.required?.includes(k) ? rs(t) : `Option<${rs(t)}>`}`).join(', ')} }`:''},`;}).join('\n')}\n}\n`;
 } else if(s.properties) rust+=`${derives}\n#[serde(deny_unknown_fields)]\npub struct ${n} {\n${Object.entries(s.properties).map(([k,t])=>` ${!s.required?.includes(k) ? '#[serde(default, skip_serializing_if = "Option::is_none")]\n' : ''} pub ${k === "type" ? "r#type" : k}: ${s.required?.includes(k) ? rs(t) : `Option<${rs(t)}>`},`).join('\n')}\n}\n`;
}
const variants=defs.Command.oneOf;
rust+=`impl Command { pub fn capability(&self) -> Capability { match self { ${variants.map(s=>{let n=title(s.properties.type.const);return `Self::${n}${Object.keys(s.properties).length>1?' { .. }':''} => Capability::${title(s['x-capability'])},`;}).join('\n')} } } }\n`;
rust=execFileSync('rustfmt',['--emit','stdout','--edition','2021'],{input:rust,encoding:'utf8'});
const catalog=variants.map(s=>({id:s.properties.type.const,label:s.title ?? title(s.properties.type.const),result_schema:s["x-result"],capability:s['x-capability'],schema:s}));
// Share repeated data objects without changing schemas or adding reference depth.
// The frontend reads this immutable graph; canonical JSON remains authoritative.
const occurrences = new Map();
function countData(value) {
 if (!value || typeof value !== 'object') return;
 const key = JSON.stringify(value);
 occurrences.set(key, (occurrences.get(key) ?? 0) + 1);
 for (const child of Object.values(value)) countData(child);
}
countData(defs);
const shared = [...occurrences].filter(([key, count]) => count > 1 && key.length >= 50)
 .sort(([a], [b]) => a.length - b.length || a.localeCompare(b));
const names = new Map(shared.map(([key], index) => [key, `s${index}`]));
function emitData(value, own) {
 if (!value || typeof value !== 'object') return JSON.stringify(value);
 const name = names.get(JSON.stringify(value));
 if (name && name !== own) return name;
 if (Array.isArray(value)) return `[${value.map(child => emitData(child)).join(',')}]`;
 return `{${Object.entries(value).map(([key, child]) => `${JSON.stringify(key)}:${emitData(child)}`).join(',')}}`;
}
const browserSchema = '// Generated. Edit contracts/v2.schema.json and run the generator.\n'
 + shared.map(([key]) => `const ${names.get(key)} = ${emitData(JSON.parse(key), names.get(key))};\n`).join('')
 + `export default { $defs: ${emitData(defs)} };\n`;
for(const [p,c] of [['src/contracts.rs',rust],['frontend/src/lib/generated-schema.ts',browserSchema],['frontend/src/lib/contracts.ts',types],['runtime/src/generated/contracts.ts',types],['contracts/catalog.json',JSON.stringify(catalog,null,2)+'\n']]) {
 const target=path.join(root,p);
 if(process.argv.includes('--check')) {if(!fs.existsSync(target)||fs.readFileSync(target,'utf8')!==c) throw Error(`Contract drift: ${p}`);}
 else {fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,c);}
}
