import { beforeEach, describe, expect, it } from 'vitest';
import { q } from './index';

beforeEach(() => {
  document.body.innerHTML = `
    <div id="box" style="width: 100px; height: 40px; opacity: 1;">box</div>
    <div id="hidden" style="display: none;">hidden</div>`;
});

function opacityOf(id: string): string {
  const el = document.getElementById(id) as HTMLElement;
  return el.style.opacity || getComputedStyle(el).opacity;
}

describe('effects', () => {
  it('fadeOut fades then hides; fadeIn restores', async () => {
    const box = document.getElementById('box') as HTMLElement;
    await q('#box').fadeOut(5);
    expect(box.style.display).toBe('none');
    await q('#box').fadeIn(5);
    expect(box.style.display).toBe('');
    expect(opacityOf('box')).toBe('1');
  });

  it('fadeTo reaches the target opacity', async () => {
    await q('#box').fadeTo(0.25, 5);
    expect(opacityOf('box')).toBe('0.25');
  });

  it('fadeToggle follows visibility', async () => {
    await q('#box').fadeToggle(5);
    expect((document.getElementById('box') as HTMLElement).style.display).toBe('none');
    await q('#box').fadeToggle(5);
    expect((document.getElementById('box') as HTMLElement).style.display).toBe('');
  });

  it('slideUp hides and slideDown restores', async () => {
    const box = document.getElementById('box') as HTMLElement;
    await q('#box').slideUp(5);
    expect(box.style.display).toBe('none');
    await q('#box').slideDown(5);
    expect(box.style.display).toBe('');
    expect(box.style.height).toBe('');
  });

  it('slideToggle follows visibility', async () => {
    await q('#hidden').slideToggle(5);
    expect((document.getElementById('hidden') as HTMLElement).style.display).toBe('');
  });

  it('animated show/hide resolve and set end state', async () => {
    await q('#box').hide(5);
    expect((document.getElementById('box') as HTMLElement).style.display).toBe('none');
    await q('#box').show(5);
    expect((document.getElementById('box') as HTMLElement).style.display).toBe('');
    expect(opacityOf('box')).toBe('1');
  });

  it('animate() runs raw keyframes', async () => {
    await q('#box').animate([{ opacity: '1' }, { opacity: '0.3' }], 5);
    expect(opacityOf('box')).toBe('0.3');
  });

  it('calls complete callbacks', async () => {
    let done = 0;
    await q('#box').fadeTo(0.5, {
      duration: 5,
      complete: () => {
        done++;
      },
    });
    expect(done).toBe(1);
  });

  it('stop() cancels without rejecting', async () => {
    const pending = q('#box').fadeOut(5000);
    q('#box').stop();
    await expect(pending).resolves.toBeUndefined();
  });

  it('works without WAAPI (instant fallback)', async () => {
    const box = document.getElementById('box') as HTMLElement;
    const hack = box as unknown as { animate: Element['animate'] | undefined };
    const original = hack.animate;
    hack.animate = undefined;
    try {
      await q('#box').fadeTo(0.4, 50);
      expect(opacityOf('box')).toBe('0.4');
    } finally {
      hack.animate = original;
    }
  });
});
