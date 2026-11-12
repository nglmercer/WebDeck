import fs from 'node:fs';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '../..');
const catalog = JSON.parse(fs.readFileSync(path.join(root, 'webdeck/translations/en_US.json'), 'utf8'));
const messages = Object.fromEntries(Object.entries(catalog).filter(([key]) => key.startsWith('ui_')));
if (!Object.keys(messages).length || Object.values(messages).some(value => typeof value !== 'string' || !value)) throw Error('English UI catalog must contain non-empty string values');
const output = '// Generated from webdeck/translations/en_US.json by tools/i18n/generate.mjs.\n' +
  '// Edit the JSON catalog; run npm run i18n:generate --prefix frontend.\n' +
  '// prettier-ignore\n' + `export const messages = ${JSON.stringify(messages, null, 2)} as const;\n`;
const target = path.join(root, 'frontend/src/lib/messages.ts');
if (process.argv.includes('--check')) {
  if (!fs.existsSync(target) || fs.readFileSync(target, 'utf8') !== output) {
    console.error('English fallback is stale. Run npm run i18n:generate --prefix frontend.');
    process.exitCode = 1;
  }
} else if (!fs.existsSync(target) || fs.readFileSync(target, 'utf8') !== output) {
  fs.writeFileSync(target, output);
}
