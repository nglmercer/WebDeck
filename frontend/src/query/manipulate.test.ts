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
  it('append/prepend place content first/last', () => {
    q('#root').append('<span id="tail">t</span>');
    expect(q('#root').children().last().get(0)?.id).toBe('tail');
    q('#root').prepend(document.createElement('hr'));
    expect(q('#root').children().first().get(0)?.tagName).toBe('HR');
  });

  it('before/after insert siblings', () => {
    q('#a').before('<i id="bef"></i>');
    q('#b').after('<i id="aft"></i>');
    expect(q('#a').prev().get(0)?.id).toBe('bef');
    expect(q('#b').next().get(0)?.id).toBe('aft');
  });

  it('appendTo/prependTo/insertBefore/insertAfter return the inserted set', () => {
    const moved = q('#a').appendTo('#other');
    expect(moved.length).toBe(1);
    expect(document.querySelector('#other')?.contains(moved.get(0) as Element)).toBe(true);
    q('<b id="pre">x</b>').prependTo('#root');
    expect(q('#root').children().first().get(0)?.id).toBe('pre');
    q('<i id="ib"></i>').insertBefore('#b');
    expect(q('#b').prev().get(0)?.id).toBe('ib');
    q('<i id="ia"></i>').insertAfter('#b');
    expect(q('#b').next().get(0)?.id).toBe('ia');
  });

  it('clones for all but the last target', () => {
    q('<em class="cp">x</em>').appendTo('div');
    expect(document.querySelectorAll('em.cp').length).toBe(2);
  });
});

describe('removal', () => {
  it('remove() detaches and purges data/listeners', () => {
    let fired = 0;
    const ref = document.getElementById('a');
    expect(ref).not.toBeNull();
    if (!ref) return;
    q(ref).data('k', 1).on('click', () => {
      fired++;
    });
    q(ref).remove();
    expect(document.getElementById('a')).toBeNull();
    // Re-inserted nodes come back dead (data + listeners purged).
    document.getElementById('root')?.appendChild(ref);
    expect(q(ref).data('k')).toBeUndefined();
    ref.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).toBe(0);
  });

  it('remove(selector) only removes matches', () => {
    q('p').remove('#b');
    expect(document.getElementById('a')).not.toBeNull();
    expect(document.getElementById('b')).toBeNull();
  });

  it('detach() keeps the set alive', () => {
    const kept = q('#a').detach();
    expect(document.getElementById('a')).toBeNull();
    expect(kept.length).toBe(1);
    q('#root').append(kept);
    expect(document.getElementById('a')).not.toBeNull();
  });

  it('empty() clears children', () => {
    q('#root').empty();
    expect(q('#root').children().length).toBe(0);
  });

  it('replaceWith() swaps nodes', () => {
    q('#a').replaceWith('<section id="s">s</section>');
    expect(document.getElementById('a')).toBeNull();
    expect(document.getElementById('s')?.textContent).toBe('s');
  });

  it('clone() copies markup and data, listeners only on request', () => {
    let fired = 0;
    q('#a').data('k', 'v').on('click', () => {
      fired++;
    });
    const plain = q('#a').clone();
    expect(plain.get(0)?.outerHTML).toBe(document.getElementById('a')?.outerHTML);
    expect(plain.data<string>('k')).toBe('v');
    plain.get(0)?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).toBe(0);
    const live = q('#a').clone(true);
    live.get(0)?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).toBe(1);
  });
});
