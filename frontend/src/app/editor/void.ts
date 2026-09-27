// Void-button + delete flows (extracted from editor.ts).
//
// NOTE: this module and ./events import each other (createVoidButton ends
// with reloadEditorEvents; reloadEditorEvents binds the confirmations
// below). Both edges are deferred calls inside function bodies, so the
// cycle is safe: nothing runs at module-evaluation time.

import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { q, byId } from '../../query';
import { addPlusIcon } from '../../components/icons';
import { pageState } from '../state';
import { reloadEditorEvents } from './events';
import { editorUiState } from './state';

export function formCoords(form: Element): { parentId: string; formNumber: string } {
  const parentId = (q(form).parent().prop('id') ?? '').replace(/^folder-/, '');
  const formClassMatch = (q(form).attr('class') ?? '').match(/form-(\d+)/);
  return { parentId, formNumber: formClassMatch?.[1] ?? '' };
}

export function createVoidButton(event: Event | null = null, form: Element | null = null): void {
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
  const addButtonDiv = q('<div>').addClass('add-button').css('display', 'flex');
  const checkboxDiv = q('<div>').addClass('checkbox').css('display', 'none');
  // Parsed from markup: <svg>/<path> land in the SVG namespace, exactly
  // like the createElementNS version (attribute order is irrelevant).
  const svg = q(addPlusIcon().value);

  addButtonDiv.append(svg);
  voidDiv.append(checkboxDiv).append(addButtonDiv);

  console.log(`${parentId} > ${formNumber}`);

  addButtonDiv.attr('add_FOLDER', parentId);
  addButtonDiv.attr('add_ID', formNumber);

  // Upstream also ORs `tempEditorConfig == {}`, which is always false in
  // JS (object identity), so the 1:1 condition is just the null check.
  if (pageState.tempEditorConfig === null) {
    void loadEditorConfig().then((config) => {
      pageState.tempEditorConfig = loadEditorConfigSync(config);
    });
    console.log(pageState.tempEditorConfig);
  }
  const buttons = (
    (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
      | Record<string, JsonObject[]>
      | undefined
  )?.[parentId];
  if (buttons) buttons[Number(formNumber)] = { VOID: 'VOID' } as unknown as JsonObject;
  q(closestForm).replaceWith(voidDiv);

  console.log('The button has been removed.');
  console.log(pageState.tempEditorConfig);
  pageState.config = pageState.tempEditorConfig;

  reloadEditorEvents();
}

async function loadEditorConfig(): Promise<JsonObject> {
  const response = await fetch('/get_config', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({}),
  });
  return (await response.json()) as JsonObject;
}

function loadEditorConfigSync(config: JsonObject): JsonObject {
  void config;
  // Upstream assigns the promise-returning call itself (a latent bug kept
  // shape-identical: tempEditorConfig would hold a Promise). Reproduce the
  // assignment target faithfully is impossible in TS; keep current config.
  return pageState.tempEditorConfig;
}

export function deleteFolder(folderName: string): void {
  editorUiState.ifModif = 1;
  const folderElement = byId('folder-' + folderName).get(0) ?? null;
  if (folderElement) {
    if (confirm(text('delete_folder_confirm'))) {
      q(folderElement).remove();

      const folderButtons = q(`button[onclick="folder(\`${folderName}\`)"]`);
      for (const btn of folderButtons.toArray()) {
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
    }
  }
}

export function showAddConfirmation(_event: Event): void {
  console.log('The button is being added');
}

export function showDeleteConfirmation(event: Event): void {
  // NOTE: upstream `|| force` references an undeclared global (throws only
  // when Cancel is clicked, after confirm already returned false — net
  // effect identical to falsy here).
  const force = (window as unknown as Record<string, unknown>)['force'];
  if (pageState.editorMode === 1 && (confirm(text('button_delete_confirmation')) || force)) {
    createVoidButton(event, null);
  }
}

export function showEditWindow(_event: Event): void {
  console.log('edit pressé');
}
