import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="root">
      <button id="btn" type="button">go</button>
      <ul id="list"><li class="item">a</li><li class="item">b</li></ul>
    </div>`;
});

function click(el: Element): void {
  el.dispatchEvent(new MouseEvent('click', { bubbles: true }));
}

describe('events', () => {
  it('on() subscribes with typed events and this-binding', () => {
    let seen = '';
    let tag = '';
    q('#btn').on('click', function (e) {
      seen = e.type;
      tag = this.tagName;
    });
    const btn = document.getElementById('btn');
    expect(btn).not.toBeNull();
    if (btn) click(btn);
    expect(seen).toBe('click');
    expect(tag).toBe('BUTTON');
  });

  it('supports space-separated types and custom events', () => {
    let count = 0;
    q('#btn').on('click custom', () => {
      count++;
    });
    const btn = document.getElementById('btn');
    if (btn) {
      click(btn);
      btn.dispatchEvent(new CustomEvent('custom', { bubbles: true }));
    }
    expect(count).toBe(2);
  });

  it('delegates to matching descendants', () => {
    const hits: string[] = [];
    q('#list').on('click', '.item', function () {
      hits.push(this.textContent ?? '');
    });
    const items = document.querySelectorAll('.item');
    items.forEach((li) => click(li));
    expect(hits).toEqual(['a', 'b']);
    // Non-matching clicks inside the root do not fire.
    q('#root').on('click', '.missing', () => {
      hits.push('!');
    });
    const btn = document.getElementById('btn');
    if (btn) click(btn);
    expect(hits).toEqual(['a', 'b']);
  });
});
