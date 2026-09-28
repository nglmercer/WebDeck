import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import {
  SEARCH_DROPDOWN_CHANGE,
  SearchDropdown,
  defineSearchDropdown,
  wireSearchDropdown,
} from './search-dropdown';

beforeEach(() => {
  defineSearchDropdown();
  document.body.innerHTML = '';
});

function dropdownEl(id: string, placeholder: string, extra = ''): string {
  return `<search-dropdown class="" id="${id}" placeholder="${placeholder}"${extra}></search-dropdown>`;
}

function mount(id = 'dd1', options: string[] = ['ctrl', 'enter', 'f5']): SearchDropdown {
  document.body.innerHTML = dropdownEl(id, 'Search…');
  const el = document.getElementById(id) as SearchDropdown;
  el.setOptions(options);
  return el;
}

describe('searchDropdown', () => {

  it('builds search box and options on connect', () => {
    const el = mount();
    expect(el.querySelector('input.sd-search')).not.toBeNull();
    expect(el.querySelectorAll('.sd-option')).toHaveLength(3);
  });

  it('passes input-class to the search box', () => {
    document.body.innerHTML = dropdownEl('dd1', '', ' input-class="key-aux"');
    expect(
      (document.querySelector('.sd-search') as HTMLInputElement).classList.contains('key-aux')
    ).toBe(true);
  });

  it('filters options case-insensitively', async () => {
    const el = mount('dd1', ['ctrl', 'enter', 'f5']);
    const search = el.querySelector('.sd-search') as HTMLInputElement;
    search.value = 'T';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    expect([...el.querySelectorAll('.sd-option')].map((o) => o.textContent)).toEqual([
      'ctrl',
      'enter',
    ]);
  });

  it('selects on click and emits the value', async () => {
    const el = mount();
    const seen: string[] = [];
    el.addEventListener(SEARCH_DROPDOWN_CHANGE, (event) => {
      seen.push((event as CustomEvent<string>).detail);
    });
    (el.querySelector('.sd-option[data-value="enter"]') as HTMLElement).click();
    await tick();
    expect(el.value).toBe('enter');
    expect(seen).toEqual(['enter']);
    expect(
      (el.querySelector('.sd-option[data-value="enter"]') as HTMLElement).getAttribute(
        'aria-selected'
      )
    ).toBe('true');
  });

  it('picks the first visible option on Enter', async () => {
    const el = mount('dd1', ['ctrl', 'enter', 'f5']);
    const search = el.querySelector('.sd-search') as HTMLInputElement;
    search.value = 'f';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await tick();
    expect(el.value).toBe('f5');
  });

  it('moves the highlight with arrows', async () => {
    const el = mount('dd1', ['ctrl', 'enter', 'f5']);
    const search = el.querySelector('.sd-search') as HTMLInputElement;
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await tick();
    expect(el.querySelector('.sd-option.active')?.getAttribute('data-value')).toBe('ctrl');
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await tick();
    expect(el.querySelector('.sd-option.active')?.getAttribute('data-value')).toBe('enter');
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }));
    await tick();
    expect(el.querySelector('.sd-option.active')?.getAttribute('data-value')).toBe('ctrl');
  });

  it('clears the filter on Escape', async () => {
    const el = mount('dd1', ['ctrl', 'enter']);
    const search = el.querySelector('.sd-search') as HTMLInputElement;
    search.value = 'ctrl';
    search.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    expect(el.querySelectorAll('.sd-option')).toHaveLength(1);
    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await tick();
    expect(search.value).toBe('');
    expect(el.querySelectorAll('.sd-option')).toHaveLength(2);
  });

  it('value setter accepts known options only', () => {
    const el = mount();
    el.value = 'enter';
    expect(el.value).toBe('enter');
    el.value = 'nope';
    expect(el.value).toBe('enter');
  });

  it('drops selection when options no longer contain it', () => {
    const el = mount();
    el.value = 'enter';
    el.setOptions(['f5']);
    expect(el.value).toBe('');
  });
});

describe('wireSearchDropdown', () => {
  it('mounts options and forwards selection', async () => {
    document.body.innerHTML = dropdownEl('dd1', '');
    const seen: string[] = [];
    wireSearchDropdown('dd1', ['a', 'b'], (value) => seen.push(value));
    expect(document.querySelectorAll('.sd-option')).toHaveLength(2);
    (document.querySelector('.sd-option[data-value="b"]') as HTMLElement).click();
    await tick();
    expect(seen).toEqual(['b']);
  });

  it('no-ops when the element is absent', () => {
    expect(() => wireSearchDropdown('missing', ['a'], () => {})).not.toThrow();
  });
});
