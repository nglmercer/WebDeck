import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { asArray, asObject, asString, type JsonObject } from '../framework/types';

// Repo root, resolved from this file (same approach as labels.test.ts).
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

function readRepo(relative: string): string {
  return readFileSync(join(ROOT, relative), 'utf-8');
}

const commandsJson = JSON.parse(readRepo('webdeck/commands.json')) as JsonObject;
const LANG_CODES = ['de_DE', 'en_US', 'es_ES', 'fr_FR', 'ko_KR', 'pl_PL', 'ru_RU'];

function fetchEntry(): JsonObject {
  const integrations = asObject(commandsJson['Integrations']);
  const entry = asObject(integrations['Fetch URL']);
  expect(entry, 'Integrations/Fetch URL entry exists').not.toEqual({});
  return entry;
}

describe('Fetch URL command schema', () => {
  it('maps to /fetch with marker-delimited fields', () => {
    const entry = fetchEntry();
    expect(entry['command']).toBe('/fetch');
    const args = asArray(entry['args']);
    // Hidden `text` markers interleave the inputs: the form drops empty
    // values, so the backend pairs markers (not positions) with values.
    const shape = args.map((a) => {
      const obj = asObject(a);
      return asString(obj['TYPE']) === 'text' ? asString(obj['value']) : asString(obj['TYPE']);
    });
    expect(shape).toEqual([
      'method:',
      'input dropdown',
      'url:',
      'input url',
      'headers:',
      'input headers',
      'body:',
      'input longtext',
      'timeout:',
      "input number['1','120']",
    ]);
  });

  it('hides the body field for GET and HEAD', () => {
    const entry = fetchEntry();
    const body = asObject(asArray(entry['args'])[7]);
    expect(body['visibleWhen']).toEqual({ arg: 1, notIn: ['GET', 'HEAD'] });
  });

  it('offers the seven supported methods with inline labels', () => {
    const entry = fetchEntry();
    const dropdown = asObject(asArray(entry['args'])[1]);
    expect(asArray(dropdown['options']).map((o) => asString(asObject(o)['ID']))).toEqual([
      'GET',
      'POST',
      'PUT',
      'PATCH',
      'DELETE',
      'HEAD',
      'OPTIONS',
    ]);
    for (const option of asArray(dropdown['options'])) {
      expect(asString(asObject(option)['label'])).not.toBe('');
    }
  });

  it('references a shipped button image', () => {
    const entry = fetchEntry();
    const image = asString(asObject(entry['style'])['image']);
    expect(image).not.toBe('');
    expect(existsSync(join(ROOT, 'static', 'img', image))).toBe(true);
  });

  it('has label keys in every language (no raw-key leaks)', () => {
    const keys = [
      'INTEGRATIONS_CATEGORY_NAME',
      'INTEGRATIONS_fetch_url__btn_name',
      'INTEGRATIONS_fetch_url__btn_description',
      'INTEGRATIONS_fetch_url__arg_1_name',
      'INTEGRATIONS_fetch_url__arg_2_name',
      'INTEGRATIONS_fetch_url__arg_3_name',
      'INTEGRATIONS_fetch_url__arg_4_name',
      'INTEGRATIONS_fetch_url__arg_5_name',
    ];
    for (const code of LANG_CODES) {
      const source = readRepo(`webdeck/translations/${code}.lang`);
      for (const key of keys) {
        expect(source.includes(`${key}=`), `${code} is missing ${key}`).toBe(true);
      }
    }
  });
});
