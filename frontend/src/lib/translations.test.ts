import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { createTranslator, messages } from './translations';
import { common, keys } from './translation-keys';
import { flattenMessages } from '../../locales/schema';
import type { Locale } from '../../locales/schema';
import english from '../../locales/en_US';

describe('grouped translation lookup', () => {
  it('supports typed namespace constants and dotted paths with English fallback', () => {
    const t = createTranslator(() => ({ 'common.settings': '', 'common.save': 'Guardar cambios' }));
    expect(common.save).toBe('common.save');
    expect(keys.audio.fragment).toBe('audio.fragment');
    expect(t(common.save)).toBe('Guardar cambios');
    expect(t('common.settings')).toBe('Settings');
    expect(t(keys.guides.audio.steps.wave.title)).toBe(english.guides.audio.steps.wave.title);
    expect(t.message('Unknown server error')).toBe('Unknown server error');
    expect(t.message('Save changes')).toBe('Save changes');
  });
  it('tracks replacement dictionaries and preserves interpolation and literal data', () => {
    let dictionary: Record<string, string> = { 'deck.editNamedButton': 'Editar {label}' };
    const t = createTranslator(() => dictionary);
    expect(t(keys.deck.editNamedButton, { label: '<script>literal text</script>' })).toBe(
      'Editar <script>literal text</script>',
    );
    dictionary = {
      'deck.editNamedButton': '{label} bearbeiten',
      'plugin.count': '%count% Einträge',
    };
    expect(t('deck.editNamedButton', { label: 'My button' })).toBe('My button bearbeiten');
    expect(t.message('plugin.count', { count: 2 })).toBe('2 Einträge');
    expect(t.message('plugin.count')).toBe('%count% Einträge');
  });
  it('ignores inherited properties and preserves unknown server text', () => {
    const t = createTranslator(
      () => Object.create({ 'common.save': 'Inherited' }) as Record<string, string>,
    );
    expect(t(common.save)).toBe('Save changes');
    expect(t.message('__proto__')).toBe('__proto__');
    expect(t.message('constructor')).toBe('constructor');
  });
  it('serializes nested groups to the same JSON leaf dictionary Rust merges', () => {
    const catalog = JSON.parse(readFileSync('../webdeck/translations/en_US.json', 'utf8'));
    expect(catalog).toEqual(messages);
    const partial: Locale = {
      meta: { ...english.meta, langCode: 'es' },
      common: { save: 'Guardar' },
    };
    const merged = { ...messages, ...JSON.parse(JSON.stringify(flattenMessages(partial))) };
    const t = createTranslator(() => merged);
    expect(t(common.save)).toBe('Guardar');
    expect(t(common.cancel)).toBe('Cancel');
    expect(merged['meta.langCode']).toBe('es');
    expect(Object.keys(catalog).every((key) => key.includes('.') && !key.startsWith('ui_'))).toBe(
      true,
    );
  });
  it('rejects misspelled keys and namespace objects at compile time', () => {
    const t = createTranslator(() => ({}));
    if (false) {
      // @ts-expect-error Unknown translation keys must fail type checking.
      t('common.svae');
      // @ts-expect-error A namespace is not a translatable message.
      t(keys.audio);
      // @ts-expect-error Namespace constants autocomplete only real leaves.
      t(common.missing);
    }
  });
});
