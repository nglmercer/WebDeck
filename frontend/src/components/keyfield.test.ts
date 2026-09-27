import { beforeEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import { keyField, normalizeCapturedKey, wireKeyField, NAMED_KEYS } from './keyfield';

beforeEach(() => {
  initI18n({
    key_capture: 'Capture',
    key_capture_prompt: 'Press a key…',
    key_search_keys: 'Search keys…',
  });
  document.body.innerHTML = '';
});

describe('keyField', () => {
  it('renders value input, capture button, and search dropdown', () => {
    const out = keyField({ dark: 'dark-theme', id: 'k1', value: 'a' }).value;
    expect(out).toContain('id="key-input_k1"');
    expect(out).toContain('value="a"');
    expect(out).toContain('id="key-capture_k1"');
    expect(out).toContain('>Capture</button>');
    expect(out).toContain('<search-dropdown');
    expect(out).toContain('id="key-list_k1"');
    expect(out).toContain('placeholder="Search keys…"');
    expect(out).toContain('input-class="key-aux"');
    expect(out).not.toContain('<select');
    expect(out).not.toContain('<option');
  });

  it('marks the dropdown search box so serialization skips it', () => {
    document.body.innerHTML = keyField({ dark: '', id: 'k1', value: '' }).value;
    wireKeyField('k1');
    const search = document.querySelector('#key-list_k1 .sd-search') as HTMLInputElement;
    expect(search.classList.contains('key-aux')).toBe(true);
    expect(
      (document.querySelector('#key-input_k1') as HTMLInputElement).classList.contains('key-aux')
    ).toBe(false);
  });

  it('lists every backend named key exactly once', () => {
    document.body.innerHTML = keyField({ dark: '', id: 'k1', value: '' }).value;
    wireKeyField('k1');
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
  function mount(): void {
    document.body.innerHTML = keyField({ dark: '', id: 'k1', value: '' }).value;
    wireKeyField('k1');
  }

  function input(): HTMLInputElement {
    return document.querySelector('#key-input_k1') as HTMLInputElement;
  }

  it('writes dropdown selection into the value input', () => {
    mount();
    (document.querySelector('.sd-option[data-value="enter"]') as HTMLElement).click();
    expect(input().value).toBe('enter');
  });

  it('filters the dropdown by search text', () => {
    mount();
    const search = document.querySelector('#key-list_k1 .sd-search') as HTMLInputElement;
    search.value = 'vol';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    const visible = [...document.querySelectorAll('#key-list_k1 .sd-option')].map(
      (o) => o.getAttribute('data-value')
    );
    expect(visible).toEqual(['volumemute', 'volumeup', 'volumedown']);
  });

  it('captures the next physical keypress', () => {
    mount();
    (document.querySelector('#key-capture_k1') as HTMLButtonElement).click();
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe(
      'Press a key…'
    );
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true }));
    expect(input().value).toBe('up');
    // Button restored; listener disarmed (further keys ignored).
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe('Capture');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', bubbles: true, cancelable: true }));
    expect(input().value).toBe('up');
  });

  it('cancels capture on Escape without touching the value', () => {
    mount();
    (document.querySelector('#key-capture_k1') as HTMLButtonElement).click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    expect(input().value).toBe('');
    expect((document.querySelector('#key-capture_k1') as HTMLButtonElement).textContent).toBe('Capture');
  });

  it('no-ops when the modal has no key field', () => {
    expect(() => wireKeyField('missing')).not.toThrow();
  });
});
