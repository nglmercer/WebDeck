import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="root">
      <ul id="list">
        <li class="item" id="a">a<span class="deep">x</span></li>
        <li class="item" id="b">b</li>
        <li class="item special" id="c">c</li>
      </ul>
      <p id="tail">tail</p>
    </div>`;
});

describe('traversal', () => {
  it('find() searches descendants, deduped', () => {
    expect(q('#root').find('li').length).toBe(3);
    expect(q('ul, p').find('.deep').length).toBe(1);
  });

  it('children()/parent()/parents() walk one or all levels', () => {
    expect(q('#list').children().length).toBe(3);
    expect(q('#list').children('.special').length).toBe(1);
    expect(q('#a').parent().get(0)?.id).toBe('list');
    expect(q('#a').parent('.missing').length).toBe(0);
    const ids = q('#a').parents().map((_index, el) => el.id);
    expect(ids).toContain('list');
    expect(ids).toContain('root');
    expect(q('#a').parents('ul').length).toBe(1);
  });

  it('closest() finds self or ancestors', () => {
    expect(q('#a').closest('li').get(0)?.id).toBe('a');
    expect(q('.deep').closest('ul').get(0)?.id).toBe('list');
    expect(q('#a').closest('.missing').length).toBe(0);
  });

  it('siblings() excludes self', () => {
    expect(q('#b').siblings().length).toBe(2);
    expect(q('#b').siblings('.special').length).toBe(1);
  });

  it('next()/prev() take the immediate sibling when it matches', () => {
    expect(q('#a').next().get(0)?.id).toBe('b');
    expect(q('#a').next('.special').length).toBe(0);
    expect(q('#c').prev().get(0)?.id).toBe('b');
    expect(q('#a').prev().length).toBe(0);
  });

  it('filter()/not() narrow by selector, element, set, predicate', () => {
    const items = q('.item');
    expect(items.filter('.special').length).toBe(1);
    expect(items.not('.special').length).toBe(2);
    const b = document.getElementById('b');
    expect(b ? items.filter(b).length : 0).toBe(1);
    expect(b ? items.not(b).length : 0).toBe(2);
    expect(items.filter(q('.special')).length).toBe(1);
    expect(items.filter(function () { return this.id === 'c'; }).length).toBe(1);
    expect(items.not(function () { return this.id === 'c'; }).length).toBe(2);
  });

  it('has() keeps elements containing a match', () => {
    expect(q('li').has('.deep').length).toBe(1);
    const deep = document.querySelector('.deep');
    expect(deep ? q('li').has(deep).length : 0).toBe(1);
    expect(q('li').has('.missing').length).toBe(0);
  });
});
