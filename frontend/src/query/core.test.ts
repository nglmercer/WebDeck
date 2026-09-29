import { beforeEach, describe, expect, it } from 'vitest';
import { Q, byId, q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="app">
      <input id="name" type="text" value="ada" />
      <p class="note">one</p>
      <p class="note">two</p>
    </div>`;
});

describe('factory', () => {
  it('selects by tag with precise typing', () => {
    const inputs = q('input');
    expect(inputs).toBeInstanceOf(Q);
    expect(inputs.length).toBe(1);
    // Compile-time proof: HTMLInputElement member access typechecks.
    expect(inputs.get(0)?.value).toBe('ada');
  });

  it('selects by complex selector', () => {
    expect(q('#app .note').length).toBe(2);
    expect(q('.missing').length).toBe(0);
  });

  it('accepts explicit element generics', () => {
    const notes = q<HTMLParagraphElement>('.note');
    expect(notes.get(0)?.tagName).toBe('P');
  });

  it('scopes queries to a context element', () => {
    const app = document.getElementById('app');
    expect(q('.note', app).length).toBe(2);
    expect(q('.note', document.createElement('div')).length).toBe(0);
  });

  it('scopes queries to another set', () => {
    expect(q('.note', q('#app')).length).toBe(2);
  });

  it('wraps elements, lists, sets, and nullish', () => {
    const input = document.getElementById('name');
    expect(q(input).length).toBe(1);
    expect(q(document.querySelectorAll('p')).length).toBe(2);
    expect(q(q('p')).length).toBe(2);
    expect(q(null).length).toBe(0);
    expect(q(undefined).length).toBe(0);
  });

  it('creates elements from HTML strings', () => {
    const made = q('<div class="fresh"><span>hi</span></div>');
    expect(made.length).toBe(1);
    expect(made.get(0)?.outerHTML).toContain('fresh');
  });

  it('does not execute scripts in created HTML', () => {
    q('<img src=x onerror="window.__evil = 1">');
    expect((window as unknown as Record<string, unknown>)['__evil']).toBeUndefined();
  });
});

describe('byId', () => {
  it('finds elements by id with explicit typing', () => {
    const name = byId<HTMLInputElement>('name');
    expect(name.length).toBe(1);
    expect(name.get(0)?.value).toBe('ada');
  });

  it('returns an empty set for missing ids', () => {
    expect(byId('nope').length).toBe(0);
  });

  it('handles ids that are invalid CSS selectors', () => {
    document.body.insertAdjacentHTML('beforeend', '<div id="0X0_submit">digit</div>');
    expect(byId('0X0_submit').text()).toBe('digit');
    expect(() => document.querySelector('#0X0_submit')).toThrow();
  });
});

describe('core set operations', () => {
  it('supports length, index access, and iteration', () => {
    const notes = q('.note');
    expect(notes.length).toBe(2);
    expect(notes[0]?.textContent).toBe('one');
    expect(notes[99]).toBeUndefined();
    expect([...notes].map((el) => el.textContent)).toEqual(['one', 'two']);
  });

  it('get() returns one element or a real array', () => {
    const notes = q('.note');
    expect(notes.get(0)?.textContent).toBe('one');
    expect(notes.get(-1)?.textContent).toBe('two');
    expect(notes.get(5)).toBeUndefined();
    expect(notes.get()).toHaveLength(2);
    expect(notes.toArray()).toHaveLength(2);
  });

  it('is() tests selector, element, set, and predicate', () => {
    const first = q(q('.note').get(0) ?? null);
    expect(first.is('.note')).toBe(true);
    expect(first.is('.missing')).toBe(false);
    const el = document.querySelector('.note');
    expect(el ? first.is(el) : false).toBe(true);
    expect(first.is(q('.note'))).toBe(true);
    expect(first.is(function () { return this.tagName === 'P'; })).toBe(true);
    expect(q('.missing').is('.note')).toBe(false);
  });
});
