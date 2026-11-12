import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
const root = path.resolve(import.meta.dirname, '../..');
const require = createRequire(path.join(root, 'frontend/package.json'));
const { parse } = require('svelte/compiler');
const errors = [];
const { readCatalogs, localeDirectory } = await import('../i18n/catalog.mjs');
const { messages: english } = await readCatalogs();
const messages = new Map(Object.entries(english));
// TypeScript rejects duplicate properties too; report them here before generated JSON can hide them.
function inspectObjects(node, file) {
  if (!node || typeof node !== 'object') return;
  if (node.type === 'ObjectExpression') {
    const seen = new Set();
    for (const property of node.properties) {
      if (property.type !== 'Property' || property.computed) continue;
      const name = property.key.name ?? property.key.value;
      if (seen.has(name)) errors.push(`${file}: duplicate key ${name}`);
      seen.add(name);
    }
  }
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) value.forEach((child) => inspectObjects(child, file));
    else if (value && typeof value === 'object') inspectObjects(value, file);
  }
}
for (const file of fs.readdirSync(localeDirectory).filter((file) => file.endsWith('.ts'))) {
  // Only the AST parser's copy is escaped; authored message values stay unchanged.
  const script = fs
    .readFileSync(path.join(localeDirectory, file), 'utf8')
    .replace(/<\/script/gi, (tag) => tag.replace('/', '\\/'));
  inspectObjects(
    parse(`<script lang="ts">${script}</script>`, { modern: true }).instance.content,
    file,
  );
}
const generated = spawnSync(
  process.execPath,
  [path.join(root, 'tools/i18n/generate.mjs'), '--check'],
  { encoding: 'utf8' },
);
if (generated.status !== 0)
  errors.push(generated.stderr.trim() || 'Generated JSON catalogues are stale');
function inspectVisibleExpression(node, file) {
  if (!node) return;
  if (node.type === 'Literal' && typeof node.value === 'string') {
    if (/[a-zA-Z]/.test(node.value) && !['JavaScript', 'JSON', 'Shell'].includes(node.value))
      errors.push(`${file}: untranslated expression at ${node.start}`);
  } else if (node.type === 'ConditionalExpression') {
    inspectVisibleExpression(node.consequent, file);
    inspectVisibleExpression(node.alternate, file);
  } else if (node.type === 'LogicalExpression') {
    inspectVisibleExpression(node.left, file);
    inspectVisibleExpression(node.right, file);
  } else if (node.type === 'TemplateLiteral') {
    for (const part of node.quasis)
      if (/[a-zA-Z]/.test(part.value.cooked ?? ''))
        errors.push(`${file}: untranslated template text at ${part.start}`);
  }
}
function walk(node, file) {
  if (!node || typeof node !== 'object') return;
  if (node.type.endsWith('Directive')) return;
  if (node.type === 'ExpressionTag') inspectVisibleExpression(node.expression, file);
  if (node.type === 'Text') {
    if (/[a-zA-Z]/.test(node.data.trim()))
      errors.push(`${file}: untranslated markup at ${node.start}`);
    return;
  }
  if (node.type === 'Attribute') {
    if (
      ['aria-label', 'title', 'placeholder', 'label', 'alt'].includes(node.name) &&
      Array.isArray(node.value)
    ) {
      for (const value of node.value) {
        if (value.type === 'ExpressionTag') inspectVisibleExpression(value.expression, file);
        if (value.type === 'Text' && /[a-zA-Z]/.test(value.data))
          errors.push(`${file}: untranslated ${node.name} at ${value.start}`);
      }
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
      for (const match of source.matchAll(/\bt\(['"]([^'"]+)['"]/g))
        if (!messages.has(match[1])) errors.push(`${relative}: unknown message key ${match[1]}`);
    }
  }
}
inventory(path.join(root, 'frontend/src'));
if (errors.length) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else console.log(`Translation check: ${messages.size} keys; resources and static markup valid`);
