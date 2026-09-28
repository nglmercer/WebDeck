import { mount, unmount } from 'svelte';
import { asObject, asString, get, type BootContext } from '../framework/types';
import { initBackgroundSetting } from '../legacy/background-setting';
import { initColorSetting } from '../legacy/color-setting';
import { initFilepath } from '../legacy/filepath';
import { initFolderpath } from '../legacy/folderpath';
import { initLoadingScreen } from '../legacy/loadingscreen';
import { initThemesSetting } from '../legacy/themes-setting';
import { initUploadFile } from '../legacy/upload-file';
import { collectAddModals, wireAddModal, wireBrowserDropdowns, wireBrowserSearch } from '../views/addbutton';
import { collectEditModals, wireEditModal } from '../views/editmodal';
import { applyHead } from '../views/shell';
import { hydrateSvgs } from '../views/svg';
import { auto_resize } from '../app/zoom';
import { installGlobals, wireApp } from '../app/wireup';
import { wireModalA11y } from '../components/studio/a11y';
import { startUsageLoop } from '../app/usage';
import App from './App.svelte';

let mountedApp: Record<string, never> | null = null;

/** Full page: mount the Svelte shell, then wire behavior (DOMContentLoaded order). */
export function renderApp(target: HTMLElement, ctx: BootContext): void {
  applyHead(ctx);
  if (mountedApp !== null) {
    void unmount(mountedApp);
    mountedApp = null;
  }
  // mount() appends; clear first so re-renders replace the previous tree
  // (the same replace semantics the old innerHTML render had).
  target.textContent = '';
  mountedApp = mount(App, { target, props: { ctx } }) as unknown as Record<string, never>;

  installGlobals();

  for (const m of collectEditModals(ctx)) {
    wireEditModal(ctx, m.editModalId, m.entry);
  }
  for (const m of collectAddModals(ctx)) {
    wireAddModal(ctx, m);
  }
  wireBrowserDropdowns();
  wireBrowserSearch();
  void hydrateSvgs(document);

  initBackgroundSetting();
  initThemesSetting();
  initColorSetting();
  initFilepath();
  initFolderpath();
  initUploadFile();
  initLoadingScreen();

  wireApp(ctx);
  wireModalA11y();

  // window-load equivalents (upstream ordering: folder first, resize, poll).
  const folders = Object.keys(asObject(get(ctx.config, 'front', 'buttons')));
  if (folders[0] !== undefined) {
    window.folder?.(folders[0]);
  }
  auto_resize();
  const reloadMs = asString(get(ctx.config, 'front', 'computer_usage_reload_time'));
  startUsageLoop(Number(reloadMs) || 3000);
}
