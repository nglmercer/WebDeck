import { beforeEach, describe, expect, it } from 'vitest';
import { filterAddBrowser } from './wireup';

function mount(): HTMLElement {
  document.body.innerHTML = `
    <div class="all-commands">
      <button class="dropdown-btn" dropdown-category="Spotify">Spotify</button>
      <div class="dropdown-container">
        <div class="addbutton-description"><p>Play music</p></div>
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="play">Play</button>
        <div class="addbutton-description"><p>Stop music</p></div>
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="stop">Stop</button>
      </div>
      <button class="dropdown-btn" dropdown-category="Display">Display</button>
      <div class="dropdown-container">
        <button class="dropdown-btn no-dropdown" dropdown-commandTag="brightness">Brightness</button>
      </div>
    </div>`;
  return document.querySelector('.all-commands') as HTMLElement;
}

function displayOf(text: string): string {
  const btn = [...document.querySelectorAll('.dropdown-btn')].find(
    (b) => (b.textContent ?? '').trim() === text
  ) as HTMLElement;
  return btn.style.display;
}

describe('filterAddBrowser', () => {
  beforeEach(mount);

  it('hides non-matching leaves and reveals their container', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'stop');
    expect(displayOf('Play')).toBe('none');
    expect(displayOf('Stop')).toBe('');
    expect(displayOf('Brightness')).toBe('none');
    // Spotify branch stays with an opened panel; Display hides entirely.
    expect(displayOf('Spotify')).toBe('');
    expect(displayOf('Display')).toBe('none');
    const panels = [...document.querySelectorAll('.dropdown-container')] as HTMLElement[];
    expect(panels[0]!.style.display).toBe('block');
    expect(panels[1]!.style.display).toBe('none');
  });

  it('matches command tags, not just labels', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'bright');
    expect(displayOf('Brightness')).toBe('');
    expect(displayOf('Display')).toBe('');
  });

  it('reveals the whole subtree on a branch-label match', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'spotify');
    expect(displayOf('Play')).toBe('');
    expect(displayOf('Stop')).toBe('');
    expect(displayOf('Display')).toBe('none');
  });

  it('hides descriptions with their button and restores all on clear', () => {
    const root = document.querySelector('.all-commands') as HTMLElement;
    filterAddBrowser(root, 'play');
    const descs = [...document.querySelectorAll('.addbutton-description')] as HTMLElement[];
    expect(descs[0]!.style.display).toBe('');
    expect(descs[1]!.style.display).toBe('none');

    filterAddBrowser(root, '  ');
    for (const el of document.querySelectorAll('.dropdown-btn, .addbutton-description')) {
      expect((el as HTMLElement).style.display).toBe('');
    }
  });
});
