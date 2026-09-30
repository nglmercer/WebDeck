// Editor event wiring (extracted from editor.ts).
//
// NOTE: see ./void for the documented deferred-only import cycle with
// that module (reloadEditorEvents binds void.ts confirmations;
// createVoidButton ends with reloadEditorEvents).

import { q, byId } from '../../query';
import { toggleEditorMode } from './mode';
import { editorUiState } from './state';
import { swapButton } from './swap';
import { showDeleteConfirmation } from './void';

const wired = new WeakSet<Element>();
export function reloadEditorEvents(): void {
  // NOTE: .add-button clicks open the modal via the div.add-button binding
  // below; .edit-button clicks via wireModals in ../modals.
  for (const el of q('.delete-button').toArray()) {
    if (wired.has(el)) continue; wired.add(el);
    q(el).on('click', showDeleteConfirmation);
  }

  const AllButtons = q('form.form').toArray().concat(q('div.void').toArray());
  for (const el of AllButtons) {
    if (wired.has(el)) continue; wired.add(el);
    q(el).on('click', swapButton);
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
}
