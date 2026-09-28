// Application entry: boot sequence mirrors the old render path — fetch
// the full page context, then render views and wire up behavior.

import { mount, unmount } from 'svelte';
import { emitAppEvent } from './app/events';
import { getJson } from './framework/api';
import { initI18n } from './framework/i18n';
import type { BootContext } from './framework/types';
import { q, byId } from './query';
import { renderApp } from './views/app';
import LoadingScreen from './views/LoadingScreen.svelte';

async function boot(): Promise<void> {
  const mountEl = byId<HTMLElement>('app').get(0);
  if (!mountEl) throw new Error('missing #app mount');

  // Loading screen first (same visual sequence as the Jinja page).
  const loading = mount(LoadingScreen, { target: mountEl, props: { svgs: [] } }) as unknown as Record<
    string,
    never
  >;

  try {
    const ctx = await getJson<BootContext>('/api/boot');
    initI18n(ctx.lang);
    void unmount(loading);
    renderApp(mountEl, ctx);
    emitAppEvent('boot:ready', ctx);
  } catch (error) {
    void unmount(loading);
    q(mountEl).html(`<p style="color:white">Failed to load WebDeck: ${String(error)}</p>`);
    throw error;
  }
}

void boot();
