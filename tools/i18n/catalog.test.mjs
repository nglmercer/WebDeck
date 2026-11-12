import { test } from 'node:test';
import assert from 'node:assert/strict';
import { flattenMessages, readCatalogs, placeholders } from './catalog.mjs';

test('shared serializer preserves nested leaves, Unicode and interpolation through JSON', () => {
  const catalog = { common: { save: 'Español = 日本語', help: 'one\ntwo {name} %count%', literal: '</script>' } };
  const flat = JSON.parse(JSON.stringify(flattenMessages(catalog)));
  assert.deepEqual(flat, {
    'common.save': 'Español = 日本語',
    'common.help': 'one\ntwo {name} %count%',
    'common.literal': '</script>',
  });
  assert.equal(placeholders(flat['common.help']), 'count,name');
  assert.equal(catalog.common.save, 'Español = 日本語');
});

test('catalogues reject ambiguous keys, unsafe properties and non-string leaves', () => {
  for (const catalog of [
    { common: { save: '' } },
    { common: { save: 42 } },
    { common: [] },
    { 'common.save': 'Save' },
    JSON.parse('{"common":{"__proto__":"value"}}'),
    { common: { constructor: 'value' } },
    { common: null },
    { common: new Date() },
  ])
    assert.throws(() => flattenMessages(catalog));
});

test('every authored locale is a valid partial dictionary with no retired keys', async () => {
  const { catalogs, messages } = await readCatalogs();
  assert(catalogs.has('en_US'));
  assert(catalogs.has('es_ES'));
  assert.equal(messages['common.save'], 'Save changes');
  for (const catalog of catalogs.values()) {
    const flat = flattenMessages(catalog);
    assert.equal(typeof flat['meta.langCode'], 'string');
    assert(Object.keys(flat).every((key) => key.includes('.') && !key.startsWith('ui_')));
    const merged = { ...messages, ...flat };
    assert.equal(Object.keys(merged).length, Object.keys(messages).length);
  }
});
