import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import CloseIcon from './CloseIcon.svelte';
import DeleteXIcon from './DeleteXIcon.svelte';
import FolderDeleteIcon from './FolderDeleteIcon.svelte';
import PencilIcon from './PencilIcon.svelte';
import PlusIcon from './PlusIcon.svelte';
import SectionIcon from './SectionIcon.svelte';
import SvgSlot from './SvgSlot.svelte';
import TrashIcon from './TrashIcon.svelte';
import { addPlusIcon, SECTION_ICONS } from './icons';

// Parsed-DOM parity: upstream SVGs carry a duplicated `class` attribute
// (the parser keeps the first) and pre-escaped handler strings; the
// components render the parsed form natively.

let apps: Record<string, never>[] = [];

afterEach(async () => {
  for (const app of apps) await unmount(app);
  apps = [];
  document.body.innerHTML = '';
});

async function render(component: unknown, props: Record<string, unknown>): Promise<HTMLElement> {
  const host = document.createElement('div');
  document.body.appendChild(host);
  apps.push(
    mount(component as never, { target: host, props }) as unknown as Record<string, never>
  );
  await tick();
  return host;
}

describe('icons', () => {
  it('renders the shared modal close glyph with modal class + theme', async () => {
    const el = await render(CloseIcon, { cls: 'config-modal', dark: 'dark-theme' });
    const svg = el.querySelector('svg')!;
    expect(svg.getAttribute('class')).toBe('config-modal dark-theme');
    expect(svg.getAttribute('width')).toBe('19');
    expect(svg.getAttribute('viewBox')).toBe('0 0 16 16');
    expect(svg.querySelector('path')?.getAttribute('d')).toContain('M16 8A8 8 0 1 1 0 8');
  });

  it('renders the add-slot plus glyph with currentColor strokes', async () => {
    const el = await render(PlusIcon, {});
    const path = el.querySelector('path')!;
    expect(path.getAttribute('d')).toBe('M12 4v16m8-8H4');
    expect(path.getAttribute('stroke')).toBe('currentColor');
  });

  it('renders the tile badge glyphs', async () => {
    const pencil = await render(PencilIcon, {});
    expect(pencil.querySelector('g')?.getAttribute('id')).toBe('Edit / Edit_Pencil_01');
    expect(pencil.querySelector('path')?.getAttribute('stroke')).toBe('currentColor');

    const x = await render(DeleteXIcon, {});
    expect(x.querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 32 32');
    expect(x.querySelector('path')?.getAttribute('d')).toContain('M18.8,16l5.5-5.5');
  });

  it('uses accessible callbacks without compiling folder names as code', async () => {
    const el = await render(FolderDeleteIcon, { folderName: 'a"b', safeFolder: 'a&quot;b' });
    const svg = el.querySelector('svg.delete-icon')!;
    // The parsed attribute holds `&quot;` (callers pre-replace `"`,
    // exactly as the inline template did after its double escape).
    expect(svg.getAttribute('onclick')).toBeNull();
    expect(svg.getAttribute('role')).toBe('button');
    expect(svg.getAttribute('tabindex')).toBe('0');
    expect(svg.querySelector('path')?.getAttribute('d')).toContain('M11.742 4.258');
    expect(svg.querySelector('title')?.textContent).toBe('Delete folder a"b');
    expect(svg.getAttribute('aria-label')).toBe('Delete folder a"b');
  });

  it('renders the trash glyph with a spaced title', async () => {
    const el = await render(TrashIcon, { title: 'Remove' });
    const svg = el.querySelector('svg.choose-bg-delete-button')!;
    expect(svg.querySelector('title')?.textContent).toBe(' Remove ');
    expect(svg.querySelectorAll('path')).toHaveLength(2);
  });

  it('renders svg slot markers for the hydrator', async () => {
    const el = await render(SvgSlot, { slot: 7 });
    const span = el.querySelector('span[data-svg-slot="7"]') as HTMLElement;
    expect(span?.style.display).toBe('contents');
  });

  it('renders the browser group glyphs as currentColor strokes', async () => {
    for (const name of ['text', 'play', 'plus'] as const) {
      expect(SECTION_ICONS[name]).toContain('<path d="M');
      const el = await render(SectionIcon, { name });
      const svg = el.querySelector('svg.wd2-section-icon')!;
      expect(svg.getAttribute('stroke')).toBe('currentColor');
      expect(svg.querySelector('path')).not.toBeNull();
    }
  });

  it('keeps the runtime plus string for void-slot construction', () => {
    expect(addPlusIcon().value).toBe(
      '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24">' +
        '<path d="M12 4v16m8-8H4" stroke="currentColor" stroke-width="2" ' +
        'stroke-linecap="round" stroke-linejoin="round"/></svg>'
    );
  });
});
