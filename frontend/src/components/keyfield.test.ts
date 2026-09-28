import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import { defineKeyField, normalizeCapturedKey, wireKeyField, NAMED_KEYS } from './keyfield';

beforeEach(() => {
  initI18n({
    key_capture: 'Capture',
    key_capture_prompt: 'Press a key…',
    key_search_keys: 'Search keys…',
  });
  defineKeyField();
  document.body.innerHTML = '';
});

function keyFieldEl(dark: string, id: string, value: string): string {
  return `<key-field id="key-field_${id}" field-id="${id}" dark="${dark}" value="${value}"></key-field>`;
}

describe('keyField', () => {
  it('renders value input, capture button, and search dropdown', async () => {
    document.body.innerHTML = keyFieldEl('dark-theme', 'k1', 'a');
    await tick();
    expect(document.querySelector('#key-input_k1')).not.toBeNull();
    expect((document.querySelector('#key-input_k1') as HTMLInputElement).value).toBe('a');
    expect(document.querySelector('#key-capture_k1')?.textContent).toBe('Capture');
    expect(document.querySelector('search-dropdown')).not.toBeNull();
    const list = document.querySelector('#key-list_k1') as HTMLElement;
    expect(list.getAttribute('placeholder')).toBe('Search keys…');
    expect(list.getAttribute('input-class')).toBe('key-aux');
    expect(document.querySelector('select')).toBeNull();
    expect(document.querySelector('option')).toBeNull();
  });

  it('marks the dropdown search box so serialization skips it', async () => {
    document.body.innerHTML = keyFieldEl('', 'k1', '');
    wireKeyField('k1');
    await tick();
    const search = document.querySelector('#key-list_k1 .sd-search') as HTMLInputElement;
    expect(search.classList.contains('key-aux')).toBe(true);
    expect(
      (document.querySelector('#key-input_k1') as HTMLInputElement).classList.contains('key-aux')
    ).toBe(false);
  });

  it('lists every backend named key exactly once', async () => {
    document.body.innerHTML = keyFieldEl('', 'k1', '');
    wireKeyField('k1');
    await tick();
    expect(new Set(NAMED_KEYS).size).toBe(NAMED_KEYS.length);
    const rendered = [
      ...document.querySelectorAll('#key-list_k1 .sd-option'),
    ].map((el) => el.getAttribute('data-value'));
    expect(rendered).toEqual(NAMED_KEYS);
  });
});

describe('normalizeCapturedKey', () => {
  it('maps special keys to backend names', () => {
    expect(normalizeCapturedKey(' ')).toBe('space');
    expect(normalizeCapturedKey('Enter')).toBe('enter');
    expect(normalizeCapturedKey('ArrowUp')).toBe('up');
    expect(normalizeCapturedKey('ArrowLeft')).toBe('left');
    expect(normalizeCapturedKey('Meta')).toBe('win');
    expect(normalizeCapturedKey('Control')).toBe('ctrl');
    expect(normalizeCapturedKey('Tab')).toBe('tab');
    expect(normalizeCapturedKey('F5')).toBe('f5');
    expect(normalizeCapturedKey('F12')).toBe('f12');
    expect(normalizeCapturedKey('AudioVolumeMute')).toBe('volumemute');
  });

  it('passes printable characters through verbatim', () => {
    expect(normalizeCapturedKey('a')).toBe('a');
    expect(normalizeCapturedKey('A')).toBe('A');
    expect(normalizeCapturedKey('7')).toBe('7');
  });

  it('cancels on Escape and lowercases the rest', () => {
    expect(normalizeCapturedKey('Escape')).toBeNull();
    expect(normalizeCapturedKey('Process')).toBe('process');
  });
});

describe('wireKeyField', () => {
  async function mount(): Promise<void> {
    document.body.innerHTML = keyFieldEl('', 'k1', '');
    wireKeyField('k1');
    await tick();
  }

  function input(): HTMLInputElement {
    return document.querySelector('#key-input_k1') as HTMLInputElement;
  }

  it('writes dropdown selection into the value input', async () => {
    await mount();
    (document.querySelector('.sd-option[data-value="enter"]') as HTMLElement).click();
    expect(input().value).toBe('enter');
  });

  it('filters the dropdown by search text', async () => {
    await mount();
    const search = document.querySelector('#key-list_k1 .sd-search') as HTMLInputElement;
    search.value = 'vol';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    const visible = [...document.querySelectorAll('#key-list_k1 .sd-option')].map(
      (o) => o.getAttribute('data-value')
    );
    expect(visible).toEqual(['volumemute', 'volumeup', 'volumedown']);
  });

  it('captures the next physical keypress', async () => {
    await mount();
    (document.querySelector('#key-capture_k1') as HTMLButtonElement).click();
    await tick();
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe(
      'Press a key…'
    );
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true }));
    await tick();
    expect(input().value).toBe('up');
    // Button restored; listener disarmed (further keys ignored).
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe('Capture');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', bubbles: true, cancelable: true }));
    expect(input().value).toBe('up');
  });

  it('cancels capture on Escape without touching the value', async () => {
    await mount();
    (document.querySelector('#key-capture_k1') as HTMLButtonElement).click();
    await tick();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await tick();
    expect(input().value).toBe('');
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe('Capture');
  });

  it('no-ops when the modal has no key field', () => {
    expect(() => wireKeyField('missing')).not.toThrow();
  });
});
