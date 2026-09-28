import { getJson } from '../framework/api';
import { initI18n } from '../framework/i18n';
import type { BootContext } from '../framework/types';
import { byId, q } from '../query';
import { renderApp } from '../views/app';
import { emitAppEvent } from './events';
import { resetEditorUiState } from './editor/state';
import { toggleEditorButtonsMode } from './editor/display';
import { syncEditorButtonLabel } from './editor/mode';
import { resetModalState } from './modals';

// NOTE: this module and ./editor/mode import each other (save flows call
// refreshApp; refreshApp syncs the editor label). Both edges are deferred
// calls inside function bodies, so the cycle is safe: nothing runs at
// module-evaluation time.

function currentFolderId(): string | null {
  const visible = q('.buttons-center')
    .toArray()
    .find((el) => !q(el).hasClass('invisible'));
  const id = visible ? q(visible).prop('id') : null;
  return id?.startsWith('folder-') ? id.slice('folder-'.length) : null;
}

/**
 * Re-render the app from a fresh boot context without reloading the page.
 * Replaces the old `location.reload()` / `location.href += '?edit=true'`
 * save flows: same server state, no black flash, no stacked globals.
 *
 * Preserves `pageState.editorMode` (callers set the flag before saving)
 * and the current folder; resets the transient UI state a reload used
 * to clear (swap picks, modal stack, body classes).
 */
export async function refreshApp(): Promise<void> {
  const mountEl = byId<HTMLElement>('app').get(0) ?? null;
  if (!mountEl) return;
  const folder = currentFolderId();

  const ctx = await getJson<BootContext>('/api/boot');
  initI18n(ctx.lang);

  resetEditorUiState();
  resetModalState();
  document.body.classList.remove('swap-active');
  window.history.replaceState({}, document.title, window.location.pathname);

  renderApp(mountEl, ctx);

  if (folder !== null && window.folder) {
    window.folder(folder);
  }
  toggleEditorButtonsMode();
  syncEditorButtonLabel();
  emitAppEvent('app:refreshed', ctx);
}
