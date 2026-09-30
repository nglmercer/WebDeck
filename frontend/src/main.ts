// Application entry: boot sequence mirrors the old render path — fetch
// the full page context, then render views and wire up behavior.

import { mount, unmount } from 'svelte';
import { HttpError } from './api/client';
import PairingScreen from './features/security/PairingScreen.svelte';
import { fetchBoot } from './api/config';
import { emitAppEvent } from './app/events';
import { initI18n } from './framework/i18n';
import { byId } from './query';
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
    const ctx = await fetchBoot();
    initI18n(ctx.lang);
    void unmount(loading);
    renderApp(mountEl, ctx);
    emitAppEvent('boot:ready', ctx);
  } catch (error) {
    void unmount(loading);
    if (error instanceof HttpError && (error.status === 401 || error.status === 403)) {
      let pairing: Record<string, never>;
      pairing = mount(PairingScreen, { target: mountEl, props: { retry: async () => { void unmount(pairing); mountEl.textContent = ''; await boot(); } } }) as unknown as Record<string, never>;
      return;
    }
    const message = document.createElement('p'); message.style.color = 'white';
    message.textContent = `Failed to load WebDeck: ${String(error)}`;
    mountEl.replaceChildren(message);

  }
}

void boot();
