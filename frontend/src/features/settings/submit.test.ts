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

 it('uses numbers and real arrays for typed settings', () => {
   const form = document.createElement('form');
   form.innerHTML = `<input type="number" name="front.width" value="8"><input name="front.themes" value='["a.css"]'><input name="front.background" value='["#112233"]'>`;
   expect(settingsPayload(form)).toEqual({front:{width:8,themes:['a.css'],background:['#112233']}});
 });
