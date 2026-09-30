// Void-button + delete flows (extracted from editor.ts).
//
// NOTE: this module and ./events import each other (createVoidButton ends
// with reloadEditorEvents; reloadEditorEvents binds the confirmations
// below). Both edges are deferred calls inside function bodies, so the
// cycle is safe: nothing runs at module-evaluation time.

import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { q, byId } from '../../query';
import { showConfirm } from '../../components/dialog';
import { addPlusIcon } from '../../components/icons';
import { pageState } from '../state';
import { reloadEditorEvents } from './events';
import { editorUiState } from './state';

export function formCoords(form: Element): { parentId: string; formNumber: string } {
  const parentId = (q(form).parent().prop('id') ?? '').replace(/^folder-/, '');
  const formClassMatch = (q(form).attr('class') ?? '').match(/form-(\d+)/);
  return { parentId, formNumber: formClassMatch?.[1] ?? '' };
}

function createVoidButton(event: Event | null = null, form: Element | null = null): void {
  editorUiState.ifModif = 1;

  let closestForm: Element | null;
  if (form === null && event !== null) {
    closestForm = q(event.target as Element).closest('form.form').get(0) ?? null;
    if (closestForm === null) {
      closestForm = q(event.target as Element).closest('div.void').get(0) ?? null;
    }
  } else {
    closestForm = form;
  }
  if (!closestForm) return;

  const { parentId, formNumber } = formCoords(closestForm);

  const voidDiv = q('<div>')
    .addClass('void')
    .addClass(`form-${formNumber}`)
    .attr('id', q(closestForm).prop('id') ?? '');
  const addButtonDiv = q('<div>')
    .addClass('add-button')
    .attr('data-testid', 'add-slot')
    .attr('title', text('add_a_button'))
    .css('display', 'flex');
  const checkboxDiv = q('<div>').addClass('checkbox').css('display', 'none');
  // Parsed from markup: <svg>/<path> land in the SVG namespace, exactly
  // like the createElementNS version (attribute order is irrelevant).
  const svg = q(addPlusIcon().value);

  addButtonDiv.append(svg);
  voidDiv.append(checkboxDiv).append(addButtonDiv);

  addButtonDiv.attr('data-add-folder', parentId);
  addButtonDiv.attr('data-add-id', formNumber);

  // NOTE: tempEditorConfig is always an object here (seeded at render in
  // app.ts, refreshed on editor entry in mode.ts) — no null fallback.
  const buttons = (
    (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
      | Record<string, JsonObject[]>
      | undefined
  )?.[parentId];
  if (buttons) buttons[Number(formNumber)] = { VOID: 'VOID' } as unknown as JsonObject;
  q(closestForm).replaceWith(voidDiv);

  pageState.config = pageState.tempEditorConfig;

  reloadEditorEvents();
}

export function deleteFolder(folderName: string): void {
  editorUiState.ifModif = 1;
  if (!byId('folder-' + folderName).get(0)) return;
  // Fire-and-forget: the string onclick caller cannot await; the removal
  // runs once the user answers. Re-query inside: the DOM may have
  // re-rendered while the prompt was open.
  void showConfirm(text('delete_folder_confirm'), { danger: true }).then((confirmed) => {
    if (!confirmed) return;
    const folderElement = byId('folder-' + folderName).get(0) ?? null;
    if (!folderElement) return;
    q(folderElement).remove();

    const folderButtons = q<HTMLButtonElement>('button[data-folder-target]').toArray().filter(button => button.dataset['folderTarget'] === folderName);
    for (const btn of folderButtons) {
      if (q(btn).hasClass('wd_button')) {
        createVoidButton(null, q(btn).parent().get(0) ?? null);
      } else {
        q(btn).remove();
      }
    }

    const buttons = (
      (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
        | Record<string, unknown>
        | undefined
    );
    if (buttons) delete buttons[folderName];
  });
}

export function showDeleteConfirmation(event: Event): void {
  if (pageState.editorMode !== 1) return;
  // Upstream escape hatch: a `force` global skips the prompt. Otherwise
  // the deletion runs once the user answers (the click caller can't await).
  const force = (window as unknown as Record<string, unknown>)['force'];
  if (force) {
    createVoidButton(event, null);
    return;
  }
  void showConfirm(text('button_delete_confirmation'), { danger: true }).then((confirmed) => {
    if (confirmed) createVoidButton(event, null);
  });
}
