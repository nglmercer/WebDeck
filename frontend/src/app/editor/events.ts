// Editor event wiring (extracted from editor.ts).
//
// NOTE: see ./void for the documented deferred-only import cycle with
// that module (reloadEditorEvents binds void.ts confirmations;
// createVoidButton ends with reloadEditorEvents).

import { q, byId } from '../../query';
import { show_addbutton_modal } from '../modals';
import { pageState } from '../state';
import { SaveExitEditor, toggleEditorMode } from './mode';
import { editorUiState } from './state';
import { swapButton, swapEditorButtonFunction } from './swap';
import { showAddConfirmation, showDeleteConfirmation, showEditWindow } from './void';

export function reloadEditorEvents(): void {
  for (const el of q('.add-button').toArray()) {
    q(el).on('click', showAddConfirmation);
  }
  for (const el of q('.edit-button').toArray()) {
    q(el).on('click', showEditWindow);
  }
  for (const el of q('.delete-button').toArray()) {
    q(el).on('click', showDeleteConfirmation);
  }

  const AllButtons = q('form.form').toArray().concat(q('div.void').toArray());
  for (const el of AllButtons) {
    q(el).on('click', swapButton);
  }

  for (const button of q('div.add-button').toArray()) {
    q(button).on('click', function () {
      if (editorUiState.swapMode !== 1) {
        const addIdValue = q(button).attr('add_ID');
        const addFolderValue = q(button).attr('add_FOLDER');
        show_addbutton_modal(addFolderValue ?? null, addIdValue ?? null);
      }
    });
  }
}

export function wireEditorChrome(): void {
  for (const el of q('.swapMode-open-folder').toArray()) {
    q(el).on('mouseover', function () {
      editorUiState.isMouseOverOpenFolder = true;
    });
    q(el).on('mouseout', function () {
      setTimeout(function () {
        editorUiState.isMouseOverOpenFolder = false;
      }, 150);
    });
  }

  q('form')
    .toArray()
    .forEach((form) => {
      q(form).on('submit', function (event) {
        if (editorUiState.swapMode === 1) {
          event.preventDefault();
        }
      });
    });

  byId('editorButton').on('click', toggleEditorMode);
  byId('exitEditorButton').on('click', toggleEditorMode);

  byId('SaveExitEditorButton').on('click', function () {
    console.log('tempEditorConfig:');
    console.log(JSON.stringify(pageState.tempEditorConfig));
    SaveExitEditor(pageState.tempEditorConfig);
  });

  byId('swapEditorButton').on('click', swapEditorButtonFunction);
}
