import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="root">
      <p id="a">alpha</p>
      <p id="b">beta</p>
    </div>
    <div id="other"></div>`;
});

describe('html/text', () => {
  it('reads first html and combined text', () => {
    expect(q('#a').html()).toBe('alpha');
    expect(q('p').text()).toBe('alphabeta');
    expect(q('.missing').html()).toBeUndefined();
    expect(q('.missing').text()).toBe('');
  });

  it('writes html/text, supporting functions', () => {
    q('#a').html('<b>bold</b>');
    expect(document.querySelector('#a b')?.textContent).toBe('bold');
    q('#b').text('plain');
    expect(document.querySelector('#b')?.innerHTML).toBe('plain');
    q('p').text((index, old) => `${index}:${old}`);
    expect(q('#a').text()).toBe('0:bold');
  });
});

describe('insertion', () => {
  it('append() places content last', () => {
    q('#root').append('<span id="tail">t</span>');
    const root = document.getElementById('root');
    expect(root?.lastElementChild?.id).toBe('tail');
    q('#root').append(document.createElement('hr'));
    expect(root?.lastElementChild?.tagName).toBe('HR');
  });

  it('clones for all but the last target', () => {
    q('#root, #other').append('<em class="cp">x</em>');
    expect(document.querySelectorAll('em.cp').length).toBe(2);
  });
});

describe('removal', () => {
  it('remove() detaches and purges listeners', () => {
    let fired = 0;
    const ref = document.getElementById('a');
    expect(ref).not.toBeNull();
    if (!ref) return;
    q(ref).on('click', () => {
      fired++;
    });
    q(ref).remove();
    expect(document.getElementById('a')).toBeNull();
    // Re-inserted nodes come back dead (listeners purged).
    document.getElementById('root')?.appendChild(ref);
    ref.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).toBe(0);
  });

  it('remove(selector) only removes matches', () => {
    q('p').remove('#b');
    expect(document.getElementById('a')).not.toBeNull();
    expect(document.getElementById('b')).toBeNull();
  });

  it('replaceWith() swaps nodes', () => {
    q('#a').replaceWith('<section id="s">s</section>');
    expect(document.getElementById('a')).toBeNull();
    expect(document.getElementById('s')?.textContent).toBe('s');
  });
});
