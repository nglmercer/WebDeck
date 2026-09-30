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
import { toggleEditorButtonsMode } from './display';
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

let entryGeneration = 0;

export function toggleEditorMode(): void {
  const generation = ++entryGeneration;
  pageState.editorMode = pageState.editorMode === 0 ? 1 : 0;
  toggleEditorButtonsMode();
  syncEditorButtonLabel();
  emitAppEvent('editor:changed', { mode: pageState.editorMode });

  if (pageState.editorMode === 1) {
    // NOTE: /get_config always answers 200 — a failure leaves the seeded
    // config untouched (same as before: only a response reset it).
    fetchConfig()
      .then(function (configData: JsonObject) {
        if (generation === entryGeneration && pageState.editorMode === 1 && !editorUiState.ifModif) pageState.tempEditorConfig = structuredClone(configData);
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

let savingEditor = false;
export function SaveExitEditor(tempConfig: JsonObject): void {
  if (savingEditor) return;
  const modified = editorUiState.ifModif === 1 || editorUiState.swapChanges.length !== 0;
  const finish = (): void => {
    pageState.editorMode = 0;
    editorUiState.swapMode = 0;
    editorUiState.ifModif = 0;
    editorUiState.swapChanges = [];
    document.body.classList.remove('swap-active');
    toggleEditorButtonsMode();
    syncEditorButtonLabel();
  };
  if (!modified) { finish(); return; }
  savingEditor = true;
  saveButtonsOnly(tempConfig).then(async () => {
    finish();
    emitAppEvent('save:completed', { flow: 'buttons' });
    await refreshApp();
    await showAlert(text('settings_save_success'));
  }).catch((error: Error) => showError(error.message))
    .finally(() => { savingEditor = false; });
}
