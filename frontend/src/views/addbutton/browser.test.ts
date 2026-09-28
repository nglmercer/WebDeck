import { describe, expect, it } from 'vitest';
import type { BootContext, JsonObject } from '../../framework/types';
import { addBrowserData } from './browser';

function ctx(commands: JsonObject): BootContext {
  return { commands, dark_theme: '' } as unknown as BootContext;
}

const COMMANDS: JsonObject = {
  System: {
    Open: { command: '/start', style: {} },
    'Shutdown PC': { command: '/PCshutdown', style: { image: '', image_size: '70%' } },
    Volume: {
      TYPE: 'multiple',
      commands: [{ command: '/volume +', style: { image: 'volume-up.svg', image_size: '75%' } }],
    },
  },
  Display: {
    CPU: { command: "/usage '", style: { image: '', image_size: '' } },
    Memory: {
      TYPE: 'multiple',
      commands: [{ command: "/usage '", style: {} }],
    },
  },
  Plugins: {
    Frobnicate: { command: '/frobnicate' },
    Stuff: { TYPE: 'multiple', commands: [{ command: '/x' }] },
  },
};

describe('addBrowserData icons', () => {
  it('maps stock categories to glyphs, plugins to grid', () => {
    const { categories } = addBrowserData(ctx(COMMANDS));
    expect(categories.map((c) => c.icon)).toEqual(['sliders', 'chart', 'grid']);
  });

  it('resolves leaf art from provided, registry, and device sources', () => {
    const { categories } = addBrowserData(ctx(COMMANDS));
    const system = categories[0]!.items;
    expect(system[0]).toMatchObject({
      kind: 'single',
      leaf: { icon: { kind: 'img', src: 'static/img/folder.png', fill: '' } },
    });
    expect(system[1]).toMatchObject({ kind: 'single', leaf: { icon: { kind: 'svg' } } });
    const display = categories[1]!.items;
    expect(display[0]).toMatchObject({ kind: 'single', leaf: { icon: { kind: 'svg' } } });
  });

  it('resolves branch icons and sub-leaf icons', () => {
    const { categories } = addBrowserData(ctx(COMMANDS));
    const volume = categories[0]!.items[2]!;
    expect(volume).toMatchObject({
      kind: 'multi',
      branch: { icon: { kind: 'glyph', name: 'speaker' } },
    });
    if (volume.kind === 'multi') {
      expect(volume.branch.subs[0]!.icon.kind).toBe('svg');
    }
    const memory = categories[1]!.items[1]!;
    expect(memory).toMatchObject({ kind: 'multi', branch: { icon: { kind: 'svg' } } });
    if (memory.kind === 'multi') {
      expect(memory.branch.subs[0]!.icon.kind).toBe('svg');
    }
  });

  it('falls back to plus/grid glyphs for art-less plugin rows', () => {
    const { categories } = addBrowserData(ctx(COMMANDS));
    const plugins = categories[2]!.items;
    expect(plugins[0]).toMatchObject({
      kind: 'single',
      leaf: { icon: { kind: 'glyph', name: 'plus' } },
    });
    expect(plugins[1]).toMatchObject({
      kind: 'multi',
      branch: { icon: { kind: 'glyph', name: 'grid' } },
    });
  });
});
