import { describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { gridView } from './grid';

function testCtx(buttons: JsonObject, showNames = true): BootContext {
  initI18n({});
  return {
    config: { front: { buttons, show_names: showNames, names_color: '' }, settings: {} },
    commands: {},
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: '',
  };
}

const keyButton = { message: '/key a', name: 'a', image: 'key.png', image_size: '75%' };
const volumeButton = {
  message: '/volume + ',
  name: 'Subir el volumen',
  image: 'volume-up.svg',
  image_size: '75%',
};

describe('gridView key labels', () => {
  // The per-button edit modal renders its own preview labels (always with
  // an id); only id-less labels are the grid's below-tile names.
  function gridLabels(out: string): string[] {
    return out.match(/<p class="buttontext"(?! id=)/g) ?? [];
  }

  it('centers the key name inside press-key tiles instead of below them', () => {
    const out = gridView(testCtx({ index: [keyButton] })).value;
    expect(out).toContain('<span class="buttontext-inside">a</span>');
    expect(gridLabels(out)).toHaveLength(0);
  });

  it('keeps the below-tile name for ordinary buttons', () => {
    const out = gridView(testCtx({ index: [volumeButton] })).value;
    expect(gridLabels(out)).toHaveLength(1);
    expect(out).not.toContain('buttontext-inside');
  });

  it('falls back to the below-tile name when a key button has no image', () => {
    const out = gridView(
      testCtx({ index: [{ message: '/key a', name: 'a', image: '' }] })
    ).value;
    expect(gridLabels(out)).toHaveLength(1);
    expect(out).not.toContain('buttontext-inside');
  });

  it('shows no name anywhere when show_names is off', () => {
    const out = gridView(testCtx({ index: [keyButton] }, false)).value;
    expect(out).not.toContain('buttontext-inside');
    expect(gridLabels(out)).toHaveLength(0);
  });
});

describe('gridView editor chrome', () => {
  it('renders badge glyphs in currentColor so badges control contrast', () => {
    const out = gridView(testCtx({ index: [volumeButton] })).value;
    expect(out).toContain('stroke="currentColor"');
    expect(out).toContain('<svg fill="currentColor"');
  });

  it('renders void add-buttons without unitless offsets', () => {
    const out = gridView(testCtx({ index: [{ VOID: 'VOID' }] })).value;
    expect(out).toContain('class="add-button"');
    expect(out).not.toContain('40.3675');
  });
});
