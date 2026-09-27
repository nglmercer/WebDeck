import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="a" class="one two" style="width: 10px; opacity: 0.5;">a</div>
    <span id="b" style="display: none;">b</span>`;
});

describe('classes', () => {
  it('adds, removes, toggles, and tests classes', () => {
    const a = q('#a');
    expect(a.hasClass('one')).toBe(true);
    expect(a.hasClass('nope')).toBe(false);
    expect(q('.missing').hasClass('one')).toBe(false);
    a.addClass('three four').addClass(['five']);
    expect(document.getElementById('a')?.classList.contains('five')).toBe(true);
    a.removeClass('one two');
    expect(document.getElementById('a')?.classList.contains('one')).toBe(false);
    a.toggleClass('three');
    expect(document.getElementById('a')?.classList.contains('three')).toBe(false);
    a.toggleClass('three', true);
    expect(document.getElementById('a')?.classList.contains('three')).toBe(true);
    a.removeClass();
    expect(document.getElementById('a')?.className).toBe('');
  });

  it('accepts per-element functions', () => {
    q('div').addClass((index, current) => `fn${index}-${current.length > 0 ? 'y' : 'n'}`);
    expect(document.getElementById('a')?.classList.contains('fn0-y')).toBe(true);
  });
});

describe('css', () => {
  it('reads computed values', () => {
    expect(q('#a').css('width')).toBe('10px');
    expect(q('#a').css('opacity')).toBe('0.5');
    expect(q('.missing').css('width')).toBeUndefined();
  });

  it('writes with px handling and kebab conversion', () => {
    q('#a').css('height', 20);
    expect((document.getElementById('a') as HTMLElement).style.height).toBe('20px');
    q('#a').css('opacity', 1);
    expect((document.getElementById('a') as HTMLElement).style.opacity).toBe('1');
    q('#a').css({ backgroundColor: 'red', zIndex: 5 });
    const style = (document.getElementById('a') as HTMLElement).style;
    expect(style.backgroundColor).toBe('red');
    expect(style.zIndex).toBe('5');
    q('#a').css('--brand', 'blue');
    expect(style.getPropertyValue('--brand')).toBe('blue');
  });
});

describe('visibility', () => {
  it('hides and restores display', () => {
    const a = document.getElementById('a') as HTMLElement;
    q('#a').hide();
    expect(a.style.display).toBe('none');
    q('#a').show();
    expect(a.style.display).toBe('');
  });

  it('restores a non-default pre-hide display', () => {
    const a = document.getElementById('a') as HTMLElement;
    a.style.display = 'inline-block';
    q('#a').hide().show();
    expect(a.style.display).toBe('inline-block');
  });

  it('shows elements hidden by stylesheets', () => {
    const b = document.getElementById('b') as HTMLElement;
    expect(getComputedStyle(b).display).toBe('none');
    q('#b').show();
    expect(getComputedStyle(b).display).not.toBe('none');
  });

  it('toggles with optional force', () => {
    q('#a').toggle();
    expect((document.getElementById('a') as HTMLElement).style.display).toBe('none');
    q('#a').toggle();
    expect((document.getElementById('a') as HTMLElement).style.display).toBe('');
    q('#a').toggle(true);
    expect((document.getElementById('a') as HTMLElement).style.display).toBe('');
    q('#a').toggle(false);
    expect((document.getElementById('a') as HTMLElement).style.display).toBe('none');
  });
});
