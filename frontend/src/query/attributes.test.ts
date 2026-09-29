import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <form id="f">
      <input id="name" type="text" value="ada" data-role="admin" data-count="3" data-flag="true" data-opts='{"a":1}' />
      <input id="check" type="checkbox" value="yes" />
      <select id="single"><option value="a">a</option><option value="b" selected>b</option></select>
      <select id="multi" multiple>
        <option value="x" selected>x</option><option value="y">y</option><option value="z" selected>z</option>
      </select>
      <textarea id="bio">hi</textarea>
    </form>`;
});

describe('attributes', () => {
  it('reads, writes, and removes attributes', () => {
    expect(q('#name').attr('type')).toBe('text');
    expect(q('#name').attr('missing')).toBeUndefined();
    expect(q('.missing').attr('type')).toBeUndefined();
    q('#name').attr('title', 't').attr({ 'data-x': 1, hidden: true });
    expect(document.getElementById('name')?.getAttribute('title')).toBe('t');
    expect(document.getElementById('name')?.getAttribute('data-x')).toBe('1');
    expect(document.getElementById('name')?.hasAttribute('hidden')).toBe(true);
    q('#name').attr('title', null);
    expect(document.getElementById('name')?.hasAttribute('title')).toBe(false);
    q('#name').removeAttr('data-x hidden');
    expect(document.getElementById('name')?.hasAttribute('data-x')).toBe(false);
  });
});

describe('properties', () => {
  it('gets/sets typed DOM properties', () => {
    const check = q<HTMLInputElement>('#check');
    expect(check.prop('checked')).toBe(false);
    check.prop('checked', true);
    expect((document.getElementById('check') as HTMLInputElement).checked).toBe(true);
    q<HTMLInputElement>('#name').prop({ readOnly: true });
    expect((document.getElementById('name') as HTMLInputElement).readOnly).toBe(true);
  });
});

describe('values', () => {
  it('reads text inputs, textareas, and selects', () => {
    expect(q('#name').val()).toBe('ada');
    expect(q('#bio').val()).toBe('hi');
    expect(q('#single').val()).toBe('b');
    expect(q('#multi').val()).toEqual(['x', 'z']);
    expect(q('#f').val()).toBeUndefined();
  });

  it('writes values incl. multi-select and checkables', () => {
    q('#name').val('grace');
    expect((document.getElementById('name') as HTMLInputElement).value).toBe('grace');
    q('#multi').val(['y']);
    expect(q('#multi').val()).toEqual(['y']);
    q('#check').val(['yes']);
    expect((document.getElementById('check') as HTMLInputElement).checked).toBe(true);
    q('#name').val(null);
    expect((document.getElementById('name') as HTMLInputElement).value).toBe('');
  });
});
