import { editorPersistence } from '../features/editor/persistence';
import { configureFolders, navigateFolder } from '../features/deck/state.svelte';
import { mount, unmount } from 'svelte';
import { asObject, asString, get, type BootContext } from '../framework/types';
import { collectAddModals, wireAddModal, wireBrowserDropdowns, wireBrowserSearch } from '../views/addbutton';
import { collectEditModals, wireEditModal } from '../views/editmodal';
import { applyHead } from '../views/shell';
import { hydrateSvgs, resetSvgSlots } from '../views/svg';
import { auto_resize } from '../app/zoom';
import { wireApp } from '../app/wireup';
import { pageState } from '../app/state';
import { startUsageLoop } from '../app/usage';
import App from './App.svelte';

let mountedApp: Record<string, never> | null = null;

/** Full page: mount the Svelte shell, then wire behavior (DOMContentLoaded order). */
export function renderApp(target: HTMLElement, ctx: BootContext): void {
  pageState.canEdit = ctx.can_edit !== false;
  editorPersistence.seed(ctx.config, ctx.config_revision);
  configureFolders(Object.keys(asObject(get(ctx.config, 'front', 'buttons'))));
  applyHead(ctx);
  if (mountedApp !== null) {
    void unmount(mountedApp);
    mountedApp = null;
  }
  // mount() appends; clear first so re-renders replace the previous tree
  // (the same replace semantics the old innerHTML render had).
  target.textContent = '';
  // Slot ids are assigned during render: restart from zero so refreshes
  // never accumulate stale entries.
  resetSvgSlots();
  mountedApp = mount(App, { target, props: { ctx } }) as unknown as Record<string, never>;

  // Editable copy (temp edits mutate this; never alias the render context).
  // Seeded here so boot and refresh both get it synchronously from the
  // boot payload — no extra /get_config round-trip after every render.
  pageState.tempEditorConfig = JSON.parse(JSON.stringify(ctx.config)) as typeof ctx.config;


  for (const m of pageState.canEdit ? collectEditModals(ctx) : []) {
    wireEditModal(ctx, m.editModalId, m.entry);
  }
  for (const m of pageState.canEdit ? collectAddModals(ctx) : []) {
    wireAddModal(ctx, m);
  }
  wireBrowserDropdowns();
  wireBrowserSearch();
  void hydrateSvgs(document);

  wireApp(ctx);

  // window-load equivalents (upstream ordering: folder first, resize, poll).
  const folders = Object.keys(asObject(get(ctx.config, 'front', 'buttons')));
  if (folders[0] !== undefined) {
    navigateFolder(folders[0]);
  }
  auto_resize();
  const reloadMs = asString(get(ctx.config, 'front', 'computer_usage_reload_time'));
  startUsageLoop(Number(reloadMs) || 3000);
}
