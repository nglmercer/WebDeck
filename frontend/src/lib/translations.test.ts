import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { createTranslator } from './translations';
import { messages } from './messages';

describe('translation lookup', () => {
  it('falls back to English for missing/empty keys and leaves unknown errors readable', () => {
    const t = createTranslator(() => ({ ui_settings: '', ui_save_changes: 'Guardar cambios' }));
    expect(t('ui_settings')).toBe('Settings');
    expect(t('Save changes')).toBe('Guardar cambios');
    expect(t('Unknown server error')).toBe('Unknown server error');
    expect(t('__proto__')).toBe('__proto__');
    expect(t('constructor')).toBe('constructor');
  });
  it('tracks replacement dictionaries and interpolates placeholders without changing data', () => {
    let dictionary: Record<string, string> = { ui_edit_named_button: 'Editar {label}' };
    const t = createTranslator(() => dictionary);
    expect(t('ui_edit_named_button', { label: '<script>literal text</script>' })).toBe(
      'Editar <script>literal text</script>',
    );
    dictionary = { ui_edit_named_button: '{label} bearbeiten', legacy: '%count% Einträge' };
    expect(t('ui_edit_named_button', { label: 'My button' })).toBe('My button bearbeiten');
    expect(t('legacy', { count: 2 })).toBe('2 Einträge');
    expect(t('legacy')).toBe('%count% Einträge');
  });
  it('ships every UI key in the English resource with matching placeholders', () => {
    const lines = readFileSync('../webdeck/translations/en_US.lang', 'utf8').split('\n');
    const resource = Object.fromEntries(
      lines
        .filter((line) => line.startsWith('ui_'))
        .map((line) => {
          const separator = line.indexOf('=');
          return [line.slice(0, separator), line.slice(separator + 1)];
        }),
    );
    expect(resource).toEqual(messages);
  });
});
