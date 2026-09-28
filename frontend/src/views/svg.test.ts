import { afterEach, describe, expect, it, vi } from 'vitest';
import { hydrateSvgs, resetSvgSlots, svgInlineStyle, svgSlotId } from './svg';

describe('svgInlineStyle', () => {
  it('emits px units on both dimensions', () => {
    // A unitless height is dropped by the CSS parser: the SVG then falls
    // back to its intrinsic height (16px Bootstrap icons rendered tiny).
    expect(svgInlineStyle(87, '')).toBe('style="width:87px; height:87px; "');
  });

  it('appends the fill declaration unchanged', () => {
    expect(svgInlineStyle(59, 'fill:#fff; color:#fff;')).toBe(
      'style="width:59px; height:59px; fill:#fff; color:#fff;"'
    );
  });
});

describe('hydrateSvgs', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
    resetSvgSlots();
  });

  it('fetches each icon path once no matter how many placeholders use it', async () => {
    const fetchMock = vi.fn(async () => ({
      ok: true,
      text: async () => '<svg viewBox="0 0 1 1"></svg>',
    }));
    vi.stubGlobal('fetch', fetchMock);
    document.body.innerHTML = ['a', 'b', 'c']
      .map((cls) => {
        const id = svgSlotId('static/img/icon.svg', `class="${cls}"`);
        return `<span data-svg-slot="${id}"></span>`;
      })
      .join('');
    await hydrateSvgs(document);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    // ...while each placeholder still gets its own attributes applied.
    expect(document.querySelectorAll('svg').length).toBe(3);
    expect(document.querySelector('svg.a')).not.toBeNull();
    expect(document.querySelector('svg.c')).not.toBeNull();
  });
});
