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

  it('parent() takes the immediate parent when it matches', () => {
    expect(q('#a').parent().get(0)?.id).toBe('list');
    expect(q('#a').parent('.missing').length).toBe(0);
  });

  it('closest() finds self or ancestors', () => {
    expect(q('#a').closest('li').get(0)?.id).toBe('a');
    expect(q('.deep').closest('ul').get(0)?.id).toBe('list');
    expect(q('#a').closest('.missing').length).toBe(0);
  });

  it('next() takes the immediate sibling when it matches', () => {
    expect(q('#a').next().get(0)?.id).toBe('b');
    expect(q('#a').next('.special').length).toBe(0);
  });
});
