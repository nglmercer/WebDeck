// Swap mode: pick-two swapping, undo/redo, mode toggle (extracted from editor.ts).

import { text } from '../../framework/i18n';
import type { JsonObject } from '../../framework/types';
import { q, byId } from '../../query';
import { pageState } from '../state';
import { showError } from '../toast';
import {
  hideEditorPartially,
  setEditorButtonsDisplay,
  showEditorPartially,
  swapButtonLabel,
} from './display';
import { editorUiState } from './state';
import { formCoords, showDeleteConfirmation } from './void';

function swapForms(parentId1: string, formNumber1: string, parentId2: string, formNumber2: string): void {
  // Exact-id root + descendant search: equivalent to `#folder-X .form-Y`
  // without interpolating user-controlled folder names into a selector.
  const form1 = byId(`folder-${parentId1}`).find(`.form-${formNumber1}`).get(0) ?? null;
  const form2 = byId(`folder-${parentId2}`).find(`.form-${formNumber2}`).get(0) ?? null;
  if (!form1 || !form2) return;

  const tempHtml = q(form1).html() ?? '';
  q(form1).html(q(form2).html() ?? '');
  q(form2).html(tempHtml);

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
  q('.swap-picked').removeClass('swap-picked');
}

export function swapButton(event: Event): void {
  if (pageState.editorMode === 1 && editorUiState.swapMode === 1 && !editorUiState.isMouseOverOpenFolder) {
    // The folder chip lives inside the form now: taps on it navigate
    // instead of selecting (covers touch, where no mouseover precedes).
    if (q(event.target as Element).closest('.swapMode-open-folder').get(0)) return;
    let closestForm = q(event.target as Element).closest('form.form').get(0) ?? null;
    if (closestForm === null) {
      closestForm = q(event.target as Element).closest('div.void').get(0) ?? null;
    }
    if (!closestForm) return;
    const { parentId, formNumber } = formCoords(closestForm);

    const checkbox = q(closestForm).find('div.checkbox').get(0) ?? null;
    if (editorUiState.swapFirstBtn === 0) {
      editorUiState.swapFirstBtn = `${parentId};;;${formNumber}`;
      q(checkbox).addClass('checkbox-checked');
      q(closestForm).addClass('swap-picked');
    } else if (editorUiState.swapSecondBtn === 0) {
      if (editorUiState.swapFirstBtn === `${parentId};;;${formNumber}`) {
        editorUiState.swapFirstBtn = 0;
        q(checkbox).removeClass('checkbox-checked');
        q(closestForm).removeClass('swap-picked');
      } else {
        q(checkbox).addClass('checkbox-checked');
        editorUiState.swapSecondBtn = `${parentId};;;${formNumber}`;
        if ((editorUiState.swapFirstBtn as string | 0) !== 0) {
          const [parentId1, formNumber1] = (editorUiState.swapFirstBtn as string).split(';;;') as [string, string];
          const [parentId2, formNumber2] = (editorUiState.swapSecondBtn as string).split(';;;') as [string, string];

          swapForms(parentId1, formNumber1, parentId2, formNumber2);

          editorUiState.swapChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

          editorUiState.swapFirstBtn = 0;
          editorUiState.swapSecondBtn = 0;
          editorUiState.ifModif = 1;
        }
      }
    }
  }
}

/** Keep the swap button's short label span and full hover title in sync. */
function setSwapLabel(label: string, title: string): void {
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.textContent = label;
  }
  byId('swapEditorButton').get(0)?.setAttribute('title', `${title} (S)`);
}

function swapLabels(): [short: string, full: string] {
  return editorUiState.swapMode === 0
    ? [text('swap_buttons_short'), text('swap_buttons')]
    : [text('stop_swap_mode_short'), text('stop_swap_mode')];
}

export function swapEditorButtonFunction(): void {
  editorUiState.swapMode = editorUiState.swapMode === 0 ? 1 : 0;
  {
    const [short, full] = swapLabels();
    setSwapLabel(short, full);
  }
  editorUiState.swapFirstBtn = 0;
  editorUiState.swapSecondBtn = 0;

  if (pageState.editorMode === 1 && editorUiState.swapMode === 1) {
    setSwapLabel(text('stop_swap_mode_short'), text('stop_swap_mode'));
    hideEditorPartially();
    setEditorButtonsDisplay('.swapMode-open-folder', 'inline-flex');
    setEditorButtonsDisplay('div.checkbox', 'block');
    setEditorButtonsDisplay('#swapHint', 'inline');
    document.body.classList.add('swap-active');

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
    setSwapLabel(text('swap_buttons_short'), text('swap_buttons'));
    editorUiState.swapMode = 0;
    showEditorPartially();

    setEditorButtonsDisplay('.swapMode-open-folder', 'none');
    setEditorButtonsDisplay('div.checkbox', 'none');
    setEditorButtonsDisplay('#swapHint', 'none');
    document.body.classList.remove('swap-active');
    // Picks reset at toggle time; also clear their markers (previously a
    // picked checkbox stayed green after leaving swap mode).
    q('div.checkbox').removeClass('checkbox-checked');
    q('.swap-picked').removeClass('swap-picked');

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
  }
}

export function undoSwap(): void {
  const lastChange = editorUiState.swapChanges[editorUiState.swapChanges.length - 1];

  if (lastChange === undefined) {
    showError(text('no_changes_to_undo'));
  } else {
    const [swapFirst, swapSecond] = lastChange.split(' > ');
    const [parentId1, formNumber1] = (swapFirst as string).split(';;;') as [string, string];
    const [parentId2, formNumber2] = (swapSecond as string).split(';;;') as [string, string];

    swapForms(parentId1, formNumber1, parentId2, formNumber2);

    editorUiState.swapChanges.splice(-1);
    editorUiState.swapUNChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

    editorUiState.swapFirstBtn = 0;
    editorUiState.swapSecondBtn = 0;
    editorUiState.ifModif = 1;
  }
}

export function undoUNSwap(): void {
  const lastChange = editorUiState.swapUNChanges[editorUiState.swapUNChanges.length - 1];

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
    editorUiState.ifModif = 1;
  }
}
