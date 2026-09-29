import { beforeEach, describe, expect, it, vi } from 'vitest';
import { auto_resize, wireZoomControls } from './zoom';

beforeEach(() => {
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

function mountDeck(): { scaler: HTMLElement; content: HTMLElement } {
  document.body.innerHTML =
    '<div id="deck-scale"><div class="buttons-center"><div class="all-buttons"></div></div></div>';
  const scaler = document.getElementById('deck-scale');
  const content = document.querySelector('.all-buttons');
  if (!(scaler instanceof HTMLElement) || !(content instanceof HTMLElement)) {
    throw new Error('deck markup missing');
  }
  return { scaler, content };
}

function stubContentBox(content: HTMLElement): void {
  vi.spyOn(content, 'getBoundingClientRect').mockReturnValue({
    x: 10,
    y: 20,
    width: 400,
    height: 300,
    top: 20,
    left: 10,
    right: 410,
    bottom: 320,
    toJSON: () => ({}),
  } as DOMRect);
}

/** Rotated overhang (top < 0), then the compensated re-measure at top 0. */
function stubContentBoxPortrait(content: HTMLElement) {
  const overhang = {
    x: 2,
    y: -200,
    width: 660,
    height: 1100,
    top: -200,
    left: 2,
    right: 662,
    bottom: 900,
    toJSON: () => ({}),
  } as DOMRect;
  const compensated = { ...overhang, y: 0, top: 0, bottom: 1100 };
  const spy = vi
    .spyOn(content, 'getBoundingClientRect')
    .mockReturnValueOnce(overhang)
    .mockReturnValue(compensated);
  return spy;
}

describe('auto_resize', () => {
  it('scales #deck-scale without translate()', () => {
    const { scaler, content } = mountDeck();
    stubContentBox(content);
    auto_resize();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
  });

  it('scales about the top-center so scale-up cannot clip the top row', () => {
    const { scaler, content } = mountDeck();
    stubContentBox(content);
    auto_resize();
    // A centered origin grows the grid upward past the viewport edge.
    expect(scaler.style.getPropertyValue('transform-origin')).toBe('50% 0');
    // Important-priority beats pre-fix cached stylesheets (`center !important`).
    expect(scaler.style.getPropertyPriority('transform-origin')).toBe('important');
  });

  it('pulls a portrait-rotated overhang back on-screen before fitting', () => {
    const { scaler, content } = mountDeck();
    const shifted = stubContentBoxPortrait(content);
    vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: true } as MediaQueryList);
    auto_resize();
    const host = content.parentElement as HTMLElement;
    expect(host.style.position).toBe('relative');
    expect(host.style.top).toBe('200px');
    expect(shifted).toHaveBeenCalledTimes(2);
    expect(scaler.style.transform).toContain('scale(');
  });

  it('leaves landscape content unshifted', () => {
    const { content } = mountDeck();
    stubContentBox(content);
    vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList);
    auto_resize();
    const host = content.parentElement as HTMLElement;
    expect(host.style.position).toBe('');
    expect(host.style.top).toBe('');
  });

  it('leaves #deck-scale untouched when there is no measurable content', () => {
    document.body.innerHTML = '<div id="deck-scale"></div>';
    const scaler = document.getElementById('deck-scale');
    if (!(scaler instanceof HTMLElement)) throw new Error('scaler missing');
    auto_resize();
    expect(scaler.style.transform).toBe('');
  });
});

describe('manual zoom buttons', () => {
  it('step the auto fit without reintroducing translate()', () => {
    const { scaler, content } = mountDeck();
    stubContentBox(content);
    document.body.insertAdjacentHTML(
      'beforeend',
      '<button class="dezoom-btn"></button><button class="zoom-btn"></button>'
    );
    auto_resize();
    wireZoomControls(() => false, '');
    const dezoom = document.querySelector('.dezoom-btn');
    const zoom = document.querySelector('.zoom-btn');
    if (!(dezoom instanceof HTMLElement) || !(zoom instanceof HTMLElement)) {
      throw new Error('zoom buttons missing');
    }
    dezoom.click();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
    expect(scaler.style.getPropertyValue('transform-origin')).toBe('50% 0');
    zoom.click();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
    expect(scaler.style.getPropertyValue('transform-origin')).toBe('50% 0');
  });
});
