import fs from 'node:fs/promises';
import path from 'node:path';
import { root, readCatalogs, flattenMessages, checkGenerated } from './catalog.mjs';

const { catalogs } = await readCatalogs();
const directory = path.join(root, 'webdeck/translations');
for (const [locale, catalog] of catalogs) {
  const file = path.join(root, `webdeck/translations/${locale}.json`);
  const content = JSON.stringify(flattenMessages(catalog), null, 2) + '\n';
  if (await checkGenerated(file, content)) continue;
  if (process.argv.includes('--check')) {
    console.error(
      `${path.relative(root, file)} is stale. Run npm run i18n:generate --prefix frontend.`,
    );
    process.exitCode = 1;
  } else await fs.writeFile(file, content);
}
// This directory contains generated output only. Removing a source locale must also
// remove its exported dictionary from Rust's available-language list.
for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
  if (!entry.isFile() || !entry.name.endsWith('.json') || catalogs.has(entry.name.slice(0, -5)))
    continue;
  if (process.argv.includes('--check')) {
    console.error(`${entry.name} has no TypeScript locale source. Regenerate translations.`);
    process.exitCode = 1;
  } else await fs.unlink(path.join(directory, entry.name));
}
