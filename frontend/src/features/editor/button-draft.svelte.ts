import type { Button, Folder } from '../../lib/contracts';
import type { Editor } from './editor.svelte';
import { clone } from './editor.svelte';
import { placeButtons } from '../deck/deck';
import { appearanceOf } from './appearance';
import { id } from '../../lib/id';
export class ButtonDraft {
  button = $state<Button | null>(null);
  folder = $state('');
  private origin = '';
  private initialButton = '';
  private initialFolder = '';
  get dirty() {
    return (
      !!this.button &&
      (JSON.stringify(this.button) !== this.initialButton || this.folder !== this.initialFolder)
    );
  }
  constructor(
    private editor: () => Editor | null,
    private current: () => Folder | undefined,
  ) {}
  open(button: Button) {
    const editor = this.editor(),
      folder = this.current();
    if (!editor || !folder) return;
    const cell =
      [
        ...placeButtons(folder.buttons, editor.draft.layout.columns, editor.draft.layout.rows)
          .placed,
      ].find(([, placement]) => placement.button.id === button.id)?.[0] ?? 0;
    this.button = clone(button);
    this.button.extensions.appearance = { ...appearanceOf(button), cell };
    this.folder = this.origin = folder.id;
    this.initialButton = JSON.stringify(this.button);
    this.initialFolder = this.folder;
  }
  createNext() {
    const editor = this.editor(),
      folder = this.current();
    if (!editor || !folder) return;
    this.create(
      placeButtons(folder.buttons, editor.draft.layout.columns, editor.draft.layout.rows).firstFree,
    );
  }
  create(cell: number) {
    if (!this.editor() || !this.current()) return;
    this.folder = this.origin = this.current()!.id;
    this.button = {
      id: id(),
      label: 'New button',
      icon: '✦',
      color: '#6654e8',
      action: { type: 'command', command: { type: 'play_pause' } },
      extensions: { appearance: { cell } },
    };
    this.initialButton = '';
    this.initialFolder = this.folder;
  }
  move(delta: number) {
    if (!this.button) return;
    const cell = Math.max(0, Number(appearanceOf(this.button).cell ?? 0) + delta);
    this.button.extensions.appearance = { ...appearanceOf(this.button), cell };
  }
  commit() {
    if (!this.button) return;
    this.editor()?.applyButton(this.button, this.folder, this.origin);
    this.button = null;
  }
  duplicate() {
    if (!this.button) return;
    this.editor()?.duplicateButton(this.button, this.folder);
    this.button = null;
  }
  remove() {
    if (!this.button) return;
    this.editor()?.removeButton(this.button.id);
    this.button = null;
  }
}
