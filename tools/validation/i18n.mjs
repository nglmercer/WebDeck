import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const root = path.resolve(import.meta.dirname, '../..');
const require = createRequire(path.join(root, 'frontend/package.json'));
const { parse } = require('svelte/compiler');
const errors = [];
const messageSource = fs.readFileSync(path.join(root, 'frontend/src/lib/messages.ts'), 'utf8');
const messageAst = parse(`<script lang="ts">${messageSource}</script>`, { modern: true });
let object = messageAst.instance.content.body.find((node) => node.declaration?.type === 'VariableDeclaration').declaration.declarations[0].init;
while (object.expression) object = object.expression;
const messages = new Map(object.properties.map((property) => [property.key.name ?? property.key.value, property.value.value]));
const placeholders = (text) => [...text.matchAll(/\{([a-zA-Z0-9_]+)\}|%([a-zA-Z0-9_]+)%/g)].map((match) => match[1] ?? match[2]).sort().join(',');
for (const file of fs.readdirSync(path.join(root, 'webdeck/translations')).filter((file) => file.endsWith('.lang'))) {
  const seen = new Set();
  for (const line of fs.readFileSync(path.join(root, 'webdeck/translations', file), 'utf8').split(/\r?\n/)) {
    if (!line.startsWith('ui_')) continue;
    const separator = line.indexOf('=');
    const key = line.slice(0, separator).trim(), value = line.slice(separator + 1).trim();
    if (!messages.has(key)) errors.push(`${file}: unknown UI key ${key}`);
    if (seen.has(key)) errors.push(`${file}: duplicate UI key ${key}`);
    seen.add(key);
    if (!value || placeholders(value) !== placeholders(messages.get(key) ?? '')) errors.push(`${file}: missing/changed placeholders in ${key}`);
    if (file === 'en_US.lang' && value !== messages.get(key)) errors.push(`${file}: English copy differs for ${key}`);
  }
  if (file === 'en_US.lang') for (const key of messages.keys()) if (!seen.has(key)) errors.push(`${file}: missing ${key}`);
}
function walk(node, file) {
  if (!node || typeof node !== 'object') return;
  if (node.type === 'Text') {
    if (/[a-zA-Z]/.test(node.data.trim())) errors.push(`${file}: untranslated markup at ${node.start}`);
    return;
  }
  if (node.type === 'Attribute') {
    if (['aria-label', 'title', 'placeholder', 'label', 'alt'].includes(node.name) && Array.isArray(node.value)) {
      for (const value of node.value) if (value.type === 'Text' && /[a-zA-Z]/.test(value.data)) errors.push(`${file}: untranslated ${node.name} at ${value.start}`);
    }
    return;
  }
  for (const [key, value] of Object.entries(node)) {
    if (['expression', 'test', 'context', 'index', 'key'].includes(key)) continue;
    if (Array.isArray(value)) for (const child of value) walk(child, file);
    else if (value && typeof value === 'object' && value.type) walk(value, file);
  }
}
function inventory(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) inventory(file);
    else if (file.endsWith('.svelte')) {
      const source = fs.readFileSync(file, 'utf8');
      const relative = path.relative(root, file);
      walk(parse(source, { modern: true }).fragment, relative);
      for (const match of source.matchAll(/\bt\(['"](ui_[a-z0-9_]+)['"]/g)) if (!messages.has(match[1])) errors.push(`${relative}: unknown UI key ${match[1]}`);
    }
  }
}
inventory(path.join(root, 'frontend/src'));
if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
else console.log(`UI translation check: ${messages.size} keys; resources and static markup valid`);
