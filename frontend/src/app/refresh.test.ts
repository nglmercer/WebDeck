import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { refreshApp } from './refresh';
import { onAppEvent } from './events';
import { pageState } from './state';
import { resetEditorUiState } from './editor/state';
import { resetModalState } from './modals';
import { stopUsageLoop } from './usage';

function tile(name: string, message: string): JsonObject {
  return { name, message, image: 'folder.png', image_size: '70%' };
}

function bootCtx(buttons: JsonObject): BootContext {
  initI18n({});
  return {
    config: {
      front: {
        buttons,
        width: '8',
        height: '4',
        show_names: true,
        names_color: '',
        computer_usage_reload_time: '3000',
      },
      settings: {},
    },
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

describe('refreshApp', () => {
  let boot: BootContext;
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    boot = bootCtx({ index: [tile('Alpha', '/folder folder1')] });
    fetchMock = vi.fn(async (url: unknown) => {
      const path = String(url);
      const body = path.includes('/api/boot')
        ? boot
        : path.includes('/usage')
          ? {}
          : { config: boot.config };
      return {
        ok: true,
        status: 200,
        json: async () => body,
        text: async () => '<svg></svg>',
      };
    });
    vi.stubGlobal('fetch', fetchMock);
    pageState.editorMode = 0;
    pageState.tempEditorConfig = {};
    pageState.disconnectCount = 0;
    resetEditorUiState();
    resetModalState();
    document.body.innerHTML = '<div id="app"></div>';
  });

  afterEach(() => {
    stopUsageLoop();
    vi.useRealTimers();
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
  });

  it('re-renders from fresh boot without duplicating the tree', async () => {
    await refreshApp();
    expect(document.querySelector('#button_e0X0')).not.toBeNull();
    expect(document.body.textContent).toContain('Alpha');

    boot = bootCtx({ index: [tile('Beta', '/folder folder1')] });
    await refreshApp();
    expect(document.querySelectorAll('#button_e0X0')).toHaveLength(1);
    expect(document.body.textContent).toContain('Beta');
    expect(document.body.textContent).not.toContain('Alpha');
  });

  it('preserves editor-mode visuals across refresh', async () => {
    pageState.editorMode = 1;
    await refreshApp();
    const editButton = document.querySelector('.edit-button') as HTMLElement | null;
    expect(editButton?.style.display).toBe('flex');
    const bar = document.querySelector('#EditorButtons') as HTMLElement | null;
    expect(bar?.style.display).toBe('flex');
  });

  it('does not stack global keydown handlers', async () => {
    await refreshApp();
    await refreshApp();
    document.documentElement.dispatchEvent(new KeyboardEvent('keydown', { key: 'q', bubbles: true }));
    // Double-bound toggles would flip twice (0 -> 1 -> 0).
    expect(pageState.editorMode).toBe(1);
  });

  it('emits app:refreshed when the re-render completes', async () => {
    const seen: string[] = [];
    const off = onAppEvent('app:refreshed', () => seen.push('refreshed'));
    await refreshApp();
    off();
    expect(seen).toEqual(['refreshed']);
  });

  it('keeps a single usage poll across refreshes', async () => {
    vi.useFakeTimers();
    await refreshApp();
    await refreshApp();
    fetchMock.mockClear();
    await vi.advanceTimersByTimeAsync(3100);
    const usageCalls = fetchMock.mock.calls.filter((call) =>
      String(call[0]).includes('/usage')
    );
    expect(usageCalls).toHaveLength(1);
  });
});
