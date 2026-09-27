// Editor chrome visibility toggles (extracted from editor.ts).

import { q, byId } from '../../query';
import { pageState } from '../state';

function editorButtons(): HTMLElement | null {
  return byId<HTMLElement>('EditorButtons').get(0) ?? null;
}

function editorButtonsFolders(): HTMLElement | null {
  return byId<HTMLElement>('EditorButtons-Folders').get(0) ?? null;
}

export function setEditorButtonsDisplay(selector: string, display: string): void {
  q(selector).css('display', display);
}

export function toggleEditorButtonsMode(): void {
  const display = pageState.editorMode === 0 ? 'none' : 'flex';
  setEditorButtonsDisplay('.add-button', display);
  setEditorButtonsDisplay('.edit-button', display);
  setEditorButtonsDisplay('.delete-button', display);
  q(editorButtons()).css('display', display);
  q(editorButtonsFolders()).css('display', pageState.editorMode === 0 ? 'none' : 'block');
}

/** Swap-button label text node (qdom has no text-node API; stays native). */
export function swapButtonLabel(): ChildNode | null {
  return byId('swapEditorButton').get(0)?.childNodes[1] ?? null;
}

export function hideEditorPartially(): void {
  setEditorButtonsDisplay('.add-button', 'none');
  setEditorButtonsDisplay('.edit-button', 'none');
  setEditorButtonsDisplay('.delete-button', 'none');
}

export function showEditorPartially(): void {
  setEditorButtonsDisplay('.add-button', 'flex');
  setEditorButtonsDisplay('.edit-button', 'flex');
  setEditorButtonsDisplay('.delete-button', 'flex');
}
