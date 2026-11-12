import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { validateCoverage } from './coverage.mjs';
import { flattenMessages } from '../../frontend/locales/schema.ts';

export { flattenMessages };
export const root = path.resolve(import.meta.dirname, '../..');
export const localeDirectory = path.join(root, 'frontend/locales');
export const placeholders = (text) =>
  [...text.matchAll(/\{([a-zA-Z0-9_]+)\}|%([a-zA-Z0-9_]+)%/g)]
    .map((match) => match[1] ?? match[2])
    .sort()
    .join(',');

export async function readCatalogs() {
  const files = (await readdir(localeDirectory))
    .filter((name) => /^[a-z]{2,3}(?:[_-][a-zA-Z0-9]+)*\.ts$/.test(name))
    .sort();
  const catalogs = new Map();
  for (const file of files) {
    const { default: catalog } = await import(pathToFileURL(path.join(localeDirectory, file)).href);
    catalogs.set(path.basename(file, '.ts'), catalog);
  }
  if (!catalogs.has('en_US')) throw Error('Missing canonical English locale');
  const messages = flattenMessages(catalogs.get('en_US'));
  for (const [locale, catalog] of catalogs) {
    for (const [key, value] of Object.entries(flattenMessages(catalog))) {
      if (!Object.hasOwn(messages, key)) throw Error(`${locale}: unknown message ${key}`);
      if (placeholders(value) !== placeholders(messages[key]))
        throw Error(`${locale}: changed placeholders in ${key}`);
    }
  }
  const coverage = validateCoverage(catalogs, messages);
  const partial = coverage.filter((report) => report.status !== 'complete');
  if (partial.length)
    throw Error(
      `Shipped locales must declare complete coverage: ${partial.map((report) => report.locale).join(', ')}`,
    );
  return { catalogs, messages };
}

export async function checkGenerated(file, expected) {
  const actual = await readFile(file, 'utf8').catch((error) => {
    if (error.code === 'ENOENT') return undefined;
    throw error;
  });
  return actual === expected;
}
