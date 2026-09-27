// Application entry: boot sequence mirrors the old render path — fetch
// the full page context, then render views and wire up behavior.

import { getJson } from './framework/api';
import { initI18n } from './framework/i18n';
import type { BootContext } from './framework/types';
import { q, byId } from './query';
import { renderApp } from './views/app';
import { loadingScreen } from './views/loading';

async function boot(): Promise<void> {
  const mount = byId<HTMLElement>('app').get(0);
  if (!mount) throw new Error('missing #app mount');

  // Loading screen first (same visual sequence as the Jinja page).
  q(mount).html(loadingScreen([]).value);

  try {
    const ctx = await getJson<BootContext>('/api/boot');
    initI18n(ctx.lang);
    renderApp(mount, ctx);
  } catch (error) {
    q(mount).html(`<p style="color:white">Failed to load WebDeck: ${String(error)}</p>`);
    throw error;
  }
}

void boot();
