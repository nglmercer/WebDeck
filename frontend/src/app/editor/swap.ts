// Swap mode: pick-two swapping, undo/redo, mode toggle (extracted from editor.ts).

import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { q, byId } from '../../query';
import { pageState } from '../state';
import { showError } from '../toast';
import { setEditorButtonsDisplay } from './display';
import { hideEditorPartially, showEditorPartially } from './display';
import { swapButtonLabel } from './display';
import { editorUiState } from './state';
import { formCoords, showDeleteConfirmation, showEditWindow } from './void';

function swapForms(parentId1: string, formNumber1: string, parentId2: string, formNumber2: string): void {
  // Exact-id root + descendant search: equivalent to `#folder-X .form-Y`
  // without interpolating user-controlled folder names into a selector.
  const form1 = byId(`folder-${parentId1}`).find(`.form-${formNumber1}`).get(0) ?? null;
  const form2 = byId(`folder-${parentId2}`).find(`.form-${formNumber2}`).get(0) ?? null;
  if (!form1 || !form2) return;

  const tempHtml = q(form1).html() ?? '';
  q(form1).html(q(form2).html() ?? '');
  q(form2).html(tempHtml);

  for (const el of q('.edit-button').toArray()) {
    q(el).on('click', showEditWindow);
  }
  for (const el of q('.delete-button').toArray()) {
    q(el).on('click', showDeleteConfirmation);
  }

  const folderButtons = (
    (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
      | Record<string, JsonObject[]>
      | undefined
  );
  if (folderButtons) {
    const tempValue = folderButtons[parentId1]?.[Number(formNumber1)];
    if (folderButtons[parentId1] && folderButtons[parentId2]) {
      folderButtons[parentId1][Number(formNumber1)] = folderButtons[parentId2][
        Number(formNumber2)
      ] as JsonObject;
      folderButtons[parentId2][Number(formNumber2)] = tempValue as JsonObject;
    }
  }
  q('div.checkbox').removeClass('checkbox-checked');
}

export function swapButton(event: Event): void {
  if (pageState.editorMode === 1 && editorUiState.swapMode === 1 && !editorUiState.isMouseOverOpenFolder) {
    let closestForm = q(event.target as Element).closest('form.form').get(0) ?? null;
    if (closestForm === null) {
      closestForm = q(event.target as Element).closest('div.void').get(0) ?? null;
    }
    if (!closestForm) return;
    const { parentId, formNumber } = formCoords(closestForm);

    const checkbox = q(closestForm).find('div.checkbox').get(0) ?? null;
    if (editorUiState.swapFirstBtn === 0) {
      editorUiState.swapFirstBtn = `${parentId};;;${formNumber}`;
      console.log(`1: ${editorUiState.swapFirstBtn}\n2: ${editorUiState.swapSecondBtn}`);
      q(checkbox).addClass('checkbox-checked');
    } else if (editorUiState.swapSecondBtn === 0) {
      if (editorUiState.swapFirstBtn === `${parentId};;;${formNumber}`) {
        editorUiState.swapFirstBtn = 0;
        console.log(`1: ${editorUiState.swapFirstBtn}\n2: ${editorUiState.swapSecondBtn}`);
        q(checkbox).removeClass('checkbox-checked');
      } else {
        q(checkbox).addClass('checkbox-checked');
        editorUiState.swapSecondBtn = `${parentId};;;${formNumber}`;
        if ((editorUiState.swapFirstBtn as string | 0) !== 0) {
          console.log('both btns are selected');
          const [parentId1, formNumber1] = (editorUiState.swapFirstBtn as string).split(';;;') as [string, string];
          const [parentId2, formNumber2] = (editorUiState.swapSecondBtn as string).split(';;;') as [string, string];

          swapForms(parentId1, formNumber1, parentId2, formNumber2);

          editorUiState.swapChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);
          console.log(`swapChanges: ${editorUiState.swapChanges}`);

          editorUiState.swapFirstBtn = 0;
          editorUiState.swapSecondBtn = 0;
          console.log(`sucessfully swapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
          editorUiState.ifModif = 1;
        }
        console.log(`1: ${editorUiState.swapFirstBtn}\n2: ${editorUiState.swapSecondBtn}`);
      }
    } else {
      console.log('if you see this, it means my code is REALLY really bad. But it works, I hope.');
    }
  }
}

export function swapEditorButtonFunction(_event?: Event): void {
  void _event;
  editorUiState.swapMode = editorUiState.swapMode === 0 ? 1 : 0;
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.nodeValue =
      editorUiState.swapMode === 0 ? `[S] ${text('swap_buttons')}` : `[S] ${text('stop_swap_mode')}`;
  }
  editorUiState.swapFirstBtn = 0;
  editorUiState.swapSecondBtn = 0;
  console.log('La valeur de swapMode a été modifiée :', editorUiState.swapMode);

  if (pageState.editorMode === 1 && editorUiState.swapMode === 1) {
    const swapLabelInner = swapButtonLabel();
    if (swapLabelInner) {
      swapLabelInner.nodeValue = `[S] ${text('stop_swap_mode')}`;
    }
    hideEditorPartially();
    setEditorButtonsDisplay('.swapMode-open-folder', 'inline-flex');
    setEditorButtonsDisplay('div.checkbox', 'block');

    // Snapshot: the loop below only touches attributes, so the live
    // collection's liveness is unobservable here.
    for (const button of q('button').toArray()) {
      if (!q(button).hasClass('EditorButtons-Folder')) {
        const onclickAttr = q(button).attr('onclick');
        const onclickHandlerAttr = q(button).attr('onclickhandler');
        if (onclickHandlerAttr) {
          editorUiState.savedOnClicks[onclickHandlerAttr] = onclickAttr ?? null;
        }
        q(button).removeAttr('onclick');
      }
    }
  } else {
    const swapLabelElse = swapButtonLabel();
    if (swapLabelElse) {
      swapLabelElse.nodeValue = `[S] ${text('swap_buttons')}`;
    }
    editorUiState.swapMode = 0;
    showEditorPartially();

    setEditorButtonsDisplay('.swapMode-open-folder', 'none');
    setEditorButtonsDisplay('div.checkbox', 'none');

    for (const button of q('button').toArray()) {
      if (!q(button).hasClass('EditorButtons-Folder')) {
        const onclickHandlerAttr = q(button).attr('onclickhandler');
        const savedOnClick = onclickHandlerAttr ? editorUiState.savedOnClicks[onclickHandlerAttr] : undefined;
        if (savedOnClick) {
          q(button).attr('onclick', savedOnClick);
        }
      }
    }
    editorUiState.swapChanges = [];
    console.log(`swapChanges: ${editorUiState.swapChanges}`);
  }
}

export function undoSwap(): void {
  const lastChange = editorUiState.swapChanges[editorUiState.swapChanges.length - 1];
  console.log(`All changes: ${editorUiState.swapChanges}`);
  console.log(`Last change: ${lastChange}`);

  if (lastChange === undefined) {
    showError(text('no_changes_to_undo'));
  } else {
    const [swapFirst, swapSecond] = lastChange.split(' > ');
    const [parentId1, formNumber1] = (swapFirst as string).split(';;;') as [string, string];
    const [parentId2, formNumber2] = (swapSecond as string).split(';;;') as [string, string];

    swapForms(parentId1, formNumber1, parentId2, formNumber2);

    editorUiState.swapChanges.splice(-1);
    editorUiState.swapUNChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

    console.log(`swapUNChanges: ${editorUiState.swapChanges}`);
    editorUiState.swapFirstBtn = 0;
    editorUiState.swapSecondBtn = 0;
    console.log(`sucessfully unswapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
    editorUiState.ifModif = 1;
  }
}

export function undoUNSwap(): void {
  const lastChange = editorUiState.swapUNChanges[editorUiState.swapUNChanges.length - 1];
  console.log(`All changes: ${editorUiState.swapUNChanges}`);
  console.log(`Last change: ${lastChange}`);

  if (lastChange === undefined) {
    showError(text('no_changes_to_undo'));
  } else {
    const [swapFirst, swapSecond] = lastChange.split(' > ');
    const [parentId1, formNumber1] = (swapFirst as string).split(';;;') as [string, string];
    const [parentId2, formNumber2] = (swapSecond as string).split(';;;') as [string, string];

    swapForms(parentId1, formNumber1, parentId2, formNumber2);

    editorUiState.swapUNChanges.splice(-1);
    editorUiState.swapChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

    editorUiState.swapFirstBtn = 0;
    editorUiState.swapSecondBtn = 0;
    console.log(`sucessfully unswapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
    editorUiState.ifModif = 1;
  }
}
