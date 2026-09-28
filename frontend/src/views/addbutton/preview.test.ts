import { afterEach, describe, expect, it } from 'vitest';
import { registerPresetIcon, resetPresetIcons } from '../../components/button-icons';
import type { BootContext, JsonObject } from '../../framework/types';
import { addPreviewData } from './preview';
import type { AddModalContext } from './types';

function ctx(namesColor = ''): BootContext {
  return { config: { front: { names_color: namesColor } }, dark_theme: '' } as unknown as BootContext;
}

function mctx(over: Partial<AddModalContext> & { commandValue: JsonObject }): AddModalContext {
  return {
    argModalId: '0X0',
    category: 'System',
    command: 'Execute batch code',
    parentCommand: '',
    subId: 0,
    commandId: '/batch',
    buttonTitle: 'Batch',
    ...over,
  };
}

afterEach(resetPresetIcons);

describe('addPreviewData', () => {
  it('keeps provided art byte-identical (png + invert fill)', () => {
    const data = addPreviewData(
      ctx(),
      mctx({ commandValue: { style: { image: 'x.png', image_size: '50%', color: 'invert' } } }),
      'X'
    );
    expect(data.buttonId).toBe(true);
    expect(data.media).toEqual({
      kind: 'img',
      src: 'static/img/x.png',
      alt: null,
      removeOnError: false,
      widthPx: 112 * (50 / 100) + 3,
      fill: 'filter: invert(1)',
    });
    expect(data.usageFill).toBe('filter: invert(1)');
  });

  it('renders svg presets through a hydrator slot', () => {
    const data = addPreviewData(
      ctx(),
      mctx({ commandValue: { style: { image: 'y.svg', image_size: '70%' } } }),
      'Y'
    );
    expect(data.media.kind).toBe('svg');
  });

  it('falls back to the registered default icon for style-less presets', () => {
    const data = addPreviewData(ctx(), mctx({ commandValue: { style: {} } }), 'Batch');
    // execscript.svg (the registered batch default) resolves like provided art.
    expect(data.buttonId).toBe(true);
    expect(data.media.kind).toBe('svg');
  });

  it('honors custom default registrations', () => {
    registerPresetIcon({ category: 'System', command: 'Execute batch code' }, {
      image: 'custom.png',
      image_size: '50%',
    });
    const data = addPreviewData(ctx(), mctx({ commandValue: {} }), 'Batch');
    expect(data.media).toEqual({
      kind: 'img',
      src: 'static/img/custom.png',
      alt: null,
      removeOnError: false,
      widthPx: 112 * (50 / 100) + 3,
      fill: '',
    });
  });

  it('keeps usage presets as text-only placeholder tiles', () => {
    const cpu = addPreviewData(
      ctx(),
      mctx({
        category: 'Display',
        command: 'CPU',
        commandId: "/usage '",
        commandValue: { style: { image: '', image_size: '' } },
      }),
      'CPU'
    );
    expect(cpu.media).toEqual({
      kind: 'img',
      src: null,
      alt: '',
      removeOnError: false,
      widthPx: 112 * (50 / 100) + 3,
      fill: '',
    });
    // Non-empty provided style still flags the tile as styled (parity).
    expect(cpu.buttonId).toBe(true);

    const ram = addPreviewData(
      ctx(),
      mctx({
        category: 'Display',
        command: 'memory usage_percent',
        parentCommand: 'Memory',
        commandId: "/usage '",
        commandValue: { style: {} },
      }),
      'RAM'
    );
    expect(ram.media.kind).toBe('img');
    expect((ram.media as { src: null }).src).toBeNull();
    expect(ram.buttonId).toBe(false);
  });
});
