// Editor mode enter/exit + save (extracted from editor.ts).

import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { byId, q } from '../../query';
import { pageState } from '../state';
import { showError } from '../toast';
import { showEditorPartially, swapButtonLabel, toggleEditorButtonsMode } from './display';
import { editorUiState } from './state';
import { swapEditorButtonFunction } from './swap';

export function toggleEditorMode(): void {
  pageState.editorMode = pageState.editorMode === 0 ? 1 : 0;
  toggleEditorButtonsMode();
  const editorButton = byId('editorButton').get(0) ?? null;
  if (editorButton) {
    q(editorButton).text(
      pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`
    );
  }
  console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);

  if (pageState.editorMode === 1) {
    fetch('/get_config')
      .then(function (response) {
        pageState.tempEditorConfig = {};
        if (response.ok) {
          return response.json();
        } else {
          throw new Error(text('settings_load_error'));
        }
      })
      .then(function (configData: JsonObject) {
        pageState.tempEditorConfig = configData;
      })
      .catch(function (error) {
        console.error(error);
      });
  } else {
    if (editorUiState.ifModif === 1) {
      console.log(`swapChanges ${editorUiState.swapChanges}`);
      location.reload();
    }
    swapEditorButtonFunction();
    pageState.editorMode = 0;
    toggleEditorButtonsMode();
    if (editorButton) {
      q(editorButton).text(
        pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`
      );
    }
    console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);
  }
}

export function SaveExitEditor(tempConfig: JsonObject): void {
  pageState.editorMode = 0;
  editorUiState.swapMode = 0;
  showEditorPartially();
  swapEditorButtonFunction();
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.nodeValue = `[S] ${text('swap_buttons')}`;
  }
  editorUiState.swapFirstBtn = 0;
  editorUiState.swapSecondBtn = 0;
  editorUiState.swapChanges = [];
  console.log(`swapChanges: ${editorUiState.swapChanges}`);
  toggleEditorButtonsMode();
  if (editorUiState.ifModif === 1 || editorUiState.swapChanges.length !== 0) {
    fetch('/save_buttons_only', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(tempConfig),
    })
      .then(function (response) {
        if (response.ok) {
          return response.json();
        } else {
          throw new Error(text('settings_save_error'));
        }
      })
      .then(function (response: { success?: boolean }) {
        if (response.success) {
          alert(text('settings_save_success'));
          location.reload();
        } else {
          showError('Error :/');
        }
      })
      .catch(function (error: Error) {
        showError(error.message);
      });
  }
  console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);
}
