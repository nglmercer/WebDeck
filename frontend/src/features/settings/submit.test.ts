import { describe, expect, it } from 'vitest';
import { settingsPayload } from './submit';
describe('settings form serialization', () => {
  it('collects booleans and dotted fields without prototype injection', () => {
    const form = document.createElement('form');
    form.innerHTML='<input name="settings.language" id="language" value="ES"><input name="front.width" value="8"><input type="checkbox" name="settings.enabled" checked><input name="__proto__.polluted" value="yes">';
    expect(settingsPayload(form)).toEqual({ settings: { language: 'es', enabled: true }, front: { width: '8' } });
    expect(({} as Record<string,unknown>)['polluted']).toBeUndefined();
  });
});
