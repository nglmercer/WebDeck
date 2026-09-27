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

  it('one() fires exactly once', () => {
    let count = 0;
    q('#btn').one('click', () => {
      count++;
    });
    const btn = document.getElementById('btn');
    if (btn) {
      click(btn);
      click(btn);
    }
    expect(count).toBe(1);
  });

  it('off() removes by type, namespace, selector, handler', () => {
    let a = 0;
    let b = 0;
    const handlerA = (): void => {
      a++;
    };
    q('#btn').on('click.ns', handlerA).on('click', () => {
      b++;
    });
    const btn = document.getElementById('btn');
    if (!btn) return;
    q('#btn').off('.ns');
    click(btn);
    expect([a, b]).toEqual([0, 1]);
    q('#btn').off('click');
    click(btn);
    expect([a, b]).toEqual([0, 1]);

    let delegated = 0;
    const root = q('#root');
    const delegate = (): void => {
      delegated++;
    };
    root.on('click', '.item', delegate);
    const item = document.querySelector('.item');
    if (item) click(item);
    expect(delegated).toBe(1);
    root.off('click', '.item', delegate);
    if (item) click(item);
    expect(delegated).toBe(1);
  });

  it('off() with no args clears everything', () => {
    let count = 0;
    q('#btn').on('click', () => {
      count++;
    });
    q('#btn').off();
    const btn = document.getElementById('btn');
    if (btn) click(btn);
    expect(count).toBe(0);
  });

  it('trigger() dispatches bubbling events with detail', () => {
    let detail: unknown = null;
    let rootSeen = false;
    q('#btn').on('ping', (e: Event) => {
      detail = (e as CustomEvent).detail;
    });
    q('#root').on('ping', () => {
      rootSeen = true;
    });
    q('#btn').trigger('ping', { n: 7 });
    expect(detail).toEqual({ n: 7 });
    expect(rootSeen).toBe(true);
  });

  it('hover() wires enter/leave', () => {
    const seen: string[] = [];
    q('#btn').hover(
      () => {
        seen.push('in');
      },
      () => {
        seen.push('out');
      }
    );
    const btn = document.getElementById('btn');
    if (btn) {
      btn.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
      btn.dispatchEvent(new MouseEvent('mouseleave', { bubbles: true }));
    }
    expect(seen).toEqual(['in', 'out']);
  });
});
