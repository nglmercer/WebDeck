import { afterEach, describe, expect, it } from 'vitest';
import {
  branchIcon,
  categoryIcon,
  iconFillStyle,
  registerBranchIcon,
  registerCategoryIcon,
  registerContrastStyle,
  registerPresetIcon,
  resetBranchIcons,
  resetCategoryIcons,
  resetContrastStyles,
  resetPresetIcons,
  resolvePresetIcon,
  rowIcon,
  seedPresetButtonState,
  type PresetIconKey,
} from './button-icons';

const BATCH: PresetIconKey = {
  category: 'System',
  command: 'Execute batch code',
  parentCommand: '',
  commandId: '/batch',
};

afterEach(() => {
  resetPresetIcons();
  resetContrastStyles();
  resetCategoryIcons();
  resetBranchIcons();
});

describe('resolvePresetIcon', () => {
  it('uses the provided image over any registered default', () => {
    expect(resolvePresetIcon(BATCH, { image: 'custom.png', image_size: '50%' })).toEqual({
      image: 'custom.png',
      image_size: '50%',
    });
  });

  it('defaults a missing provided size instead of a NaN-width tile', () => {
    expect(resolvePresetIcon(BATCH, { image: 'custom.png', image_size: '' })).toEqual({
      image: 'custom.png',
      image_size: '70%',
    });
  });

  it('resolves built-in defaults for style-less presets', () => {
    expect(resolvePresetIcon(BATCH, {})).toEqual({ image: 'execscript.svg', image_size: '70%' });
    expect(
      resolvePresetIcon(
        { category: 'System', command: 'mediacontrol next', parentCommand: 'Media control', commandId: '/mediacontrol next' },
        {}
      )
    ).toEqual({ image: 'skip-end.svg', image_size: '75%' });
    expect(
      resolvePresetIcon(
        { category: 'Spotify', command: 'x', parentCommand: '', commandId: '/spotify follow_artist' },
        {}
      )
    ).toEqual({ image: 'heart.svg', image_size: '70%' });
  });

  it('keeps a preset-declared size over the default size', () => {
    expect(resolvePresetIcon(BATCH, { image_size: '40%' })).toEqual({
      image: 'execscript.svg',
      image_size: '40%',
    });
  });

  it('resolves no icon for usage presets (text-only tiles by design)', () => {
    expect(
      resolvePresetIcon(
        { category: 'Display', command: 'CPU', parentCommand: '', commandId: "/usage '" },
        { image: '', image_size: '' }
      )
    ).toBeNull();
  });

  it('resolves the purpose-drawn defaults for power actions and websites', () => {
    expect(
      resolvePresetIcon(
        { category: 'System', command: 'Shutdown PC', parentCommand: '', commandId: '/PCshutdown' },
        { image: '', image_size: '70%' }
      )
    ).toEqual({ image: 'power.svg', image_size: '70%' });
    expect(
      resolvePresetIcon(
        { category: 'System', command: 'Open a website', parentCommand: '', commandId: '/start' },
        {}
      )
    ).toEqual({ image: 'globe.svg', image_size: '70%' });
  });

  it('resolves no icon for art-less presets without a default', () => {
    expect(
      resolvePresetIcon(
        { category: 'Plugins', command: 'Frobnicate', parentCommand: '', commandId: '/frobnicate' },
        {}
      )
    ).toBeNull();
  });

  it('prefers custom registrations over built-ins, last wins', () => {
    registerPresetIcon({ category: 'System', command: 'Execute batch code' }, {
      image: 'first.png',
      image_size: '10%',
    });
    registerPresetIcon({ category: 'System' }, { image: 'second.png', image_size: '20%' });
    expect(resolvePresetIcon(BATCH, {})).toEqual({ image: 'second.png', image_size: '20%' });
  });

  it('supports predicate matchers and global defaults', () => {
    registerPresetIcon((key) => key.commandId.startsWith('/spotify '), {
      image: 'spotify.svg',
      image_size: '60%',
    });
    // The predicate wins over the built-in follow/unfollow heart.
    expect(
      resolvePresetIcon(
        { category: 'Spotify', command: 'x', parentCommand: '', commandId: '/spotify follow_artist' },
        {}
      )
    ).toEqual({ image: 'spotify.svg', image_size: '60%' });
    // ...but provided art still wins over everything.
    expect(
      resolvePresetIcon(
        { category: 'Spotify', command: 'x', parentCommand: '', commandId: '/spotify follow_artist' },
        { image: 'mine.png', image_size: '10%' }
      )
    ).toEqual({ image: 'mine.png', image_size: '10%' });
  });

  it('supports pinning no icon for custom text-only presets', () => {
    registerPresetIcon({ commandId: '/batch' }, null);
    expect(resolvePresetIcon(BATCH, {})).toBeNull();
  });
});

describe('seedPresetButtonState', () => {
  it('seeds style-less presets with their default icon', () => {
    const button: Record<string, unknown> = { name: 'Batch' };
    seedPresetButtonState(BATCH, {}, button);
    expect(button['image']).toBe('execscript.svg');
    expect(button['image_size']).toBe('70%');
  });

  it('leaves provided art and icon-less presets untouched', () => {
    const provided: Record<string, unknown> = { image: 'mine.png', image_size: '10%' };
    seedPresetButtonState(BATCH, { image: 'mine.png', image_size: '10%' }, provided);
    expect(provided).toEqual({ image: 'mine.png', image_size: '10%' });

    const usage: Record<string, unknown> = {};
    seedPresetButtonState(
      { category: 'Display', command: 'CPU', parentCommand: '', commandId: "/usage '" },
      {},
      usage
    );
    expect(usage).toEqual({});
  });
});

describe('rowIcon', () => {
  it('shows resolved preset art, including new power defaults', () => {
    expect(rowIcon(BATCH, {})).toEqual({ kind: 'art', image: 'execscript.svg', fill: '' });
    expect(
      rowIcon(
        { category: 'System', command: 'Sleep PC', parentCommand: '', commandId: '/PCsleep' },
        { image: '', image_size: '70%' }
      )
    ).toEqual({ kind: 'art', image: 'moon.svg', fill: '' });
  });

  it('shows device glyphs for usage rows (tiles stay text-only)', () => {
    expect(
      rowIcon({ category: 'Display', command: 'CPU', parentCommand: '', commandId: "/usage '" }, {})
    ).toEqual({ kind: 'art', image: 'cpu.svg', fill: '' });
    expect(
      rowIcon(
        { category: 'Display', command: 'x', parentCommand: 'Memory', commandId: "/usage '" },
        {}
      )
    ).toEqual({ kind: 'art', image: 'memory.svg', fill: '' });
  });

  it('falls back to the plus marker for art-less plugin rows', () => {
    expect(
      rowIcon(
        { category: 'Plugins', command: 'Frobnicate', parentCommand: '', commandId: '/frobnicate' },
        {}
      )
    ).toEqual({ kind: 'glyph', name: 'plus' });
  });
});

describe('categoryIcon', () => {
  it('maps every stock category to a distinct glyph', () => {
    expect(categoryIcon('Webdeck')).toBe('grid');
    expect(categoryIcon('Display')).toBe('chart');
    expect(categoryIcon('System')).toBe('sliders');
    expect(categoryIcon('Text')).toBe('text');
    expect(categoryIcon('Utilities')).toBe('swatch');
    expect(categoryIcon('Soundboard')).toBe('speaker');
    expect(categoryIcon('OBS Studio')).toBe('video');
    expect(categoryIcon('Spotify')).toBe('music');
  });

  it('falls back to grid and honors custom registrations', () => {
    expect(categoryIcon('Plugins')).toBe('grid');
    registerCategoryIcon('Plugins', 'flask');
    expect(categoryIcon('Plugins')).toBe('flask');
  });
});

describe('branchIcon', () => {
  it('shares device art with Display leaves, glyphs elsewhere', () => {
    expect(branchIcon({ category: 'Display', branch: 'Memory' })).toEqual({
      kind: 'art',
      image: 'memory.svg',
      fill: '',
    });
    expect(branchIcon({ category: 'System', branch: 'Volume' })).toEqual({
      kind: 'glyph',
      name: 'speaker',
    });
    expect(branchIcon({ category: 'System', branch: 'Media control' })).toEqual({
      kind: 'glyph',
      name: 'play',
    });
  });

  it('falls back to grid and honors custom registrations', () => {
    expect(branchIcon({ category: 'Plugins', branch: 'Stuff' })).toEqual({
      kind: 'glyph',
      name: 'grid',
    });
    registerBranchIcon({ category: 'Plugins' }, { kind: 'glyph', name: 'flask' });
    expect(branchIcon({ category: 'Plugins', branch: 'Stuff' })).toEqual({
      kind: 'glyph',
      name: 'flask',
    });
  });
});

describe('iconFillStyle', () => {
  it('maps the invert token, tints other colors, ignores empties', () => {
    expect(iconFillStyle('invert')).toBe('filter: invert(1)');
    expect(iconFillStyle('#fff')).toBe('fill:#fff; color:#fff;');
    expect(iconFillStyle('')).toBe('');
    expect(iconFillStyle(undefined)).toBe('');
  });

  it('supports custom contrast tokens', () => {
    registerContrastStyle('auto', 'filter: contrast(2)');
    expect(iconFillStyle('auto')).toBe('filter: contrast(2)');
    resetContrastStyles();
    expect(iconFillStyle('auto')).toBe('fill:auto; color:auto;');
  });
});
