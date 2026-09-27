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

describe('auto_resize', () => {
  it('scales #deck-scale without translate()', () => {
    const { scaler, content } = mountDeck();
    stubContentBox(content);
    auto_resize();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
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
    wireZoomControls(() => false, '', '');
    const dezoom = document.querySelector('.dezoom-btn');
    const zoom = document.querySelector('.zoom-btn');
    if (!(dezoom instanceof HTMLElement) || !(zoom instanceof HTMLElement)) {
      throw new Error('zoom buttons missing');
    }
    dezoom.click();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
    zoom.click();
    expect(scaler.style.transform).toContain('scale(');
    expect(scaler.style.transform).not.toContain('translate');
  });
});
