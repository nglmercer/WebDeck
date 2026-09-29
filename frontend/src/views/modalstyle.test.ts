import { afterEach, describe, expect, it } from 'vitest';
import {
  beginModalSubmit,
  endModalSubmit,
  wireButtonNameSync,
  type ButtonState,
} from './modalstyle';

describe('modal submit guard', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('rejects missing or already-pending saves', () => {
    expect(beginModalSubmit(undefined, 'm1')).toBe(false);
    expect(beginModalSubmit({ submitPending: true }, 'm1')).toBe(false);
  });

  it('disables the submit control until the save ends', () => {
    document.body.innerHTML = '<input id="m1_submit" type="submit" />';
    const state: { submitPending?: boolean } = {};
    const submit = document.querySelector('#m1_submit') as HTMLInputElement;

    expect(beginModalSubmit(state, 'm1')).toBe(true);
    expect(state.submitPending).toBe(true);
    expect(submit.disabled).toBe(true);
    // Second attempt while in flight is rejected.
    expect(beginModalSubmit(state, 'm1')).toBe(false);

    endModalSubmit(state, 'm1');
    expect(state.submitPending).toBe(false);
    expect(submit.disabled).toBe(false);
    // A new save may begin after release.
    expect(beginModalSubmit(state, 'm1')).toBe(true);
  });

  it('tolerates a missing submit control', () => {
    const state: { submitPending?: boolean } = {};
    expect(beginModalSubmit(state, 'ghost')).toBe(true);
    expect(() => endModalSubmit(state, 'ghost')).not.toThrow();
  });
});

describe('wireButtonNameSync', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('mirrors the text input into the preview and the button state', () => {
    document.body.innerHTML = `
      <input id="button-text-input_m2" type="text" value="" />
      <span id="button-text-preview_m2"></span>`;
    const state: ButtonState = {};
    wireButtonNameSync('m2', state);

    const input = document.querySelector('#button-text-input_m2') as HTMLInputElement;
    input.value = 'Hello';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    expect(document.querySelector('#button-text-preview_m2')?.textContent).toBe('Hello');
    expect(state['name']).toBe('Hello');
  });
});
