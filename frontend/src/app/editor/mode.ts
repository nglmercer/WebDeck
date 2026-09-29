// Editor mode enter/exit + save (extracted from editor.ts).

import { saveButtonsOnly } from '../../api/buttons';
import { fetchConfig } from '../../api/config';
import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { byId, q } from '../../query';
import { pageState } from '../state';
import { emitAppEvent } from '../events';
import { refreshApp } from '../refresh';
import { showError } from '../toast';
import { showAlert } from '../../components/dialog';
import { showEditorPartially, swapButtonLabel, toggleEditorButtonsMode } from './display';
import { editorUiState } from './state';
import { swapEditorButtonFunction } from './swap';

/** Sync the config-modal editor button label with the current mode. */
export function syncEditorButtonLabel(): void {
  const editorButton = byId('editorButton').get(0) ?? null;
  if (editorButton) {
    q(editorButton).text(
      pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`
    );
  }
}

export function toggleEditorMode(): void {
  pageState.editorMode = pageState.editorMode === 0 ? 1 : 0;
  toggleEditorButtonsMode();
  syncEditorButtonLabel();
  emitAppEvent('editor:changed', { mode: pageState.editorMode });

  if (pageState.editorMode === 1) {
    // NOTE: /get_config always answers 200 — a failure leaves the seeded
    // config untouched (same as before: only a response reset it).
    fetchConfig()
      .then(function (configData: JsonObject) {
        pageState.tempEditorConfig = configData;
      })
      .catch(function (error) {
        console.error(error);
      });
  } else {
    if (editorUiState.ifModif === 1) {
      // Discard unposted temp edits by re-rendering from the server
      // (same net effect as the old location.reload()).
      void refreshApp();
      return;
    }
    swapEditorButtonFunction();
    pageState.editorMode = 0;
    toggleEditorButtonsMode();
    syncEditorButtonLabel();
  }
}

export function SaveExitEditor(tempConfig: JsonObject): void {
  pageState.editorMode = 0;
  editorUiState.swapMode = 0;
  showEditorPartially();
  swapEditorButtonFunction();
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.textContent = text('swap_buttons_short');
  }
  byId('swapEditorButton').get(0)?.setAttribute('title', `${text('swap_buttons')} (S)`);
  editorUiState.swapFirstBtn = 0;
  editorUiState.swapSecondBtn = 0;
  editorUiState.swapChanges = [];
  toggleEditorButtonsMode();
  if (editorUiState.ifModif === 1 || editorUiState.swapChanges.length !== 0) {
    saveButtonsOnly(tempConfig)
      .then(function () {
        emitAppEvent('save:completed', { flow: 'buttons' });
        void refreshApp();
        void showAlert(text('settings_save_success'));
      })
      .catch(function (error: Error) {
        // NOTE: success:false answers land here too (as the localized
        // save error instead of the old generic toast).
        showError(error.message);
      });
  }
}
