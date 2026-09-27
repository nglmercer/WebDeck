import { html, join } from '../framework/html';
import { asObject, asString, get, type BootContext } from '../framework/types';
import { q } from '../query';
import { initBackgroundSetting } from '../legacy/background-setting';
import { initColorSetting } from '../legacy/color-setting';
import { initFilepath } from '../legacy/filepath';
import { initFolderpath } from '../legacy/folderpath';
import { initLoadingScreen } from '../legacy/loadingscreen';
import { initThemesSetting } from '../legacy/themes-setting';
import { initUploadFile } from '../legacy/upload-file';
import { collectAddModals, addModalChrome, wireAddModal, wireBrowserDropdowns } from '../views/addbutton';
import { configView } from '../views/config';
import { collectEditModals, wireEditModal } from '../views/editmodal';
import { editorBarView } from '../app/editor';
import { gridView } from '../views/grid';
import { loadingScreen } from '../views/loading';
import { applyHead, shellView } from '../views/shell';
import { hydrateSvgs } from '../views/svg';
import { auto_resize } from '../app/zoom';
import { installGlobals, wireApp } from '../app/wireup';
import { startUsageLoop } from '../app/usage';

/** Full page: render all views, then wire behavior (DOMContentLoaded order). */
export function renderApp(mount: HTMLElement, ctx: BootContext): void {
  applyHead(ctx);
  q(mount).html(
    join([
      loadingScreen(ctx.svgs),
      shellView(ctx),
      // Scale wrapper: auto_resize scales ONLY the grid, so fixed UI
      // (folder bar, editor bar, modals) stays viewport-anchored instead
      // of drifting/scaling with the body transform.
      html`<div id="deck-scale">${gridView(ctx)}</div>`,
      editorBarView(),
      configView(ctx),
      addModalChrome(ctx),
    ]).value
  );

  installGlobals();

  for (const m of collectEditModals(ctx)) {
    wireEditModal(ctx, m.editModalId, m.entry, '');
  }
  for (const m of collectAddModals(ctx)) {
    wireAddModal(ctx, m);
  }
  wireBrowserDropdowns();
  void hydrateSvgs(document);

  initBackgroundSetting();
  initThemesSetting();
  initColorSetting();
  initFilepath();
  initFolderpath();
  initUploadFile();
  initLoadingScreen();

  wireApp(ctx);

  // window-load equivalents (upstream ordering: folder first, resize, poll).
  const folders = Object.keys(asObject(get(ctx.config, 'front', 'buttons')));
  if (folders[0] !== undefined) {
    window.folder?.(folders[0]);
  }
  auto_resize();
  const reloadMs = asString(get(ctx.config, 'front', 'computer_usage_reload_time'));
  startUsageLoop(Number(reloadMs) || 3000);
}
