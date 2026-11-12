import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
const root = path.resolve(import.meta.dirname, '../..');
const require = createRequire(path.join(root, 'frontend/package.json'));
const { parse } = require('svelte/compiler');
const errors = [];
const directory = path.join(root, 'webdeck/translations');
const readCatalog = (file) => {
  const source = fs.readFileSync(path.join(directory, file), 'utf8');
  const catalog = JSON.parse(source);
  if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog) || Object.values(catalog).some(value => typeof value !== 'string')) throw Error(`${file}: expected a flat string dictionary`);
  const seen = new Set();
  // The catalogs are flat JSON objects. Tokenize complete strings, including escapes,
  // so quotes inside a translated value cannot be mistaken for a property name.
  for (const token of source.matchAll(/"(?:[^"\\]|\\.)*"/g)) {
    if (!source.slice(token.index + token[0].length).trimStart().startsWith(':')) continue;
    const key = JSON.parse(token[0]);
    if (seen.has(key)) errors.push(`${file}: duplicate key ${key}`);
    seen.add(key);
  }
  return catalog;
};
const english = readCatalog('en_US.json');
const messages = new Map(Object.entries(english).filter(([key]) => key.startsWith('ui_')));
const placeholders = (text) => [...text.matchAll(/\{([a-zA-Z0-9_]+)\}|%([a-zA-Z0-9_]+)%/g)].map((match) => match[1] ?? match[2]).sort().join(',');
for (const file of fs.readdirSync(directory).filter((file) => file.endsWith('.json'))) {
  try {
    const catalog = readCatalog(file);
    for (const [key, value] of Object.entries(catalog)) {
      if (!key.startsWith('ui_')) continue;
      if (!messages.has(key)) errors.push(`${file}: unknown UI key ${key}`);
      if (!value || placeholders(value) !== placeholders(messages.get(key) ?? '')) errors.push(`${file}: missing/changed placeholders in ${key}`);
    }
  } catch (error) { errors.push(String(error)); }
}
const generated = spawnSync(process.execPath, [path.join(root, 'tools/i18n/generate.mjs'), '--check'], { encoding: 'utf8' });
if (generated.status !== 0) errors.push(generated.stderr.trim() || 'Generated English fallback is stale');
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
