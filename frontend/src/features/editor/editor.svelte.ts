export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
function sameContent(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (!left || !right || typeof left !== 'object' || typeof right !== 'object') return false;
  if (Array.isArray(left) || Array.isArray(right))
    return (
      Array.isArray(left) &&
      Array.isArray(right) &&
      left.length === right.length &&
      left.every((value, index) => sameContent(value, right[index]))
    );
  const a = left as Record<string, unknown>,
    b = right as Record<string, unknown>;
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length &&
    keys.every((key) => Object.hasOwn(b, key) && sameContent(a[key], b[key]))
  );
}
import type { ConfigResponse, Config, Button, Folder } from '../../lib/contracts';
import { placeButtons, preserveCells } from '../deck/deck';
import { id } from '../../lib/id';
import { contract } from '../../lib/schema';
import { setAppearance } from './appearance';
import { save } from '../../lib/api/client';
import { ApiError } from '../../lib/api/http';
export class Editor {
  revision = $state(0);
  draft: Config = $state(null!);
  dirty = $state(false);
  generation = $state(0);
  saving = $state(false);
  conflict = $state(false);
  failure = $state('');
  recovery = $state<Config | null>(null);
  recoveryGeneration = -1;
  private pendingSave: Promise<ConfigResponse> | null = null;
  private persisted: Config;
  constructor(snapshot: ConfigResponse) {
    this.revision = snapshot.revision;
    this.draft = clone(snapshot.config);
    this.persisted = clone(snapshot.config);
  }
  change() {
    this.generation++;
    this.dirty = !sameContent(this.draft, this.persisted);
  }
  updateAppearance(key: string, value: unknown) {
    setAppearance(this.draft.layout, key, value);
    this.change();
  }
  setDimensions(key: 'columns' | 'rows', value: number) {
    if (!Number.isInteger(value) || value < 1 || value > 128)
      throw new Error('Grid dimensions must be integers from 1 to 128.');
    this.draft.layout[key] = value;
    this.change();
  }
  addAsset(key: 'backgrounds' | 'themes', asset: string) {
    if (!this.draft.layout[key].includes(asset)) {
      this.draft.layout[key].push(asset);
      this.change();
    }
  }
  removeAsset(key: 'backgrounds' | 'themes', asset: string) {
    this.draft.layout[key] = this.draft.layout[key].filter((id) => id !== asset);
    this.change();
  }
  freezeCells() {
    for (const folder of this.draft.layout.folders)
      preserveCells(folder.buttons, this.draft.layout.columns, this.draft.layout.rows);
  }
  removeButton(buttonId: string) {
    if (
      !this.draft.layout.folders.some((folder) =>
        folder.buttons.some((button) => button.id === buttonId),
      )
    )
      return;
    this.recovery = clone(this.draft);
    this.freezeCells();
    for (const folder of this.draft.layout.folders)
      folder.buttons = folder.buttons.filter((button) => button.id !== buttonId);
    this.change();
    this.recoveryGeneration = this.generation;
  }
  applyButton(button: Button, folderId: string, originId: string) {
    contract('Button', button);
    const folder = this.draft.layout.folders.find((folder) => folder.id === folderId);
    if (!folder) throw new Error('Destination folder is unavailable');
    this.freezeCells();
    const next = clone(button);
    const appearance = (next.extensions.appearance ?? {}) as Record<string, unknown>;
    if (originId !== folderId) {
      const cell = placeButtons(
        folder.buttons,
        this.draft.layout.columns,
        this.draft.layout.rows,
      ).firstFree;
      next.extensions.appearance = { ...appearance, cell };
      const origin = this.draft.layout.folders.find((folder) => folder.id === originId);
      if (origin) origin.buttons = origin.buttons.filter((item) => item.id !== next.id);
    } else {
      const original = folder.buttons.find((item) => item.id === next.id);
      const oldCell = (original?.extensions.appearance as Record<string, unknown>)?.cell;
      if (original && oldCell !== appearance.cell) {
        const neighbor = folder.buttons.find(
          (item) =>
            item.id !== next.id &&
            (item.extensions.appearance as Record<string, unknown>)?.cell === appearance.cell,
        );
        if (neighbor)
          neighbor.extensions.appearance = {
            ...(neighbor.extensions.appearance as Record<string, unknown>),
            cell: oldCell,
          };
      }
    }
    const index = folder.buttons.findIndex((item) => item.id === next.id);
    if (index < 0) folder.buttons.push(next);
    else folder.buttons[index] = next;
    this.change();
  }
  createFolder(parentId: string): Folder {
    this.freezeCells();
    const parent = this.draft.layout.folders.find((folder) => folder.id === parentId);
    if (!parent) throw new Error('Parent folder is unavailable');
    const cell = placeButtons(
      parent.buttons,
      this.draft.layout.columns,
      this.draft.layout.rows,
    ).firstFree;
    const folder: Folder = {
      id: id(),
      label: 'New folder',
      extensions: {},
      buttons: [
        {
          id: id(),
          label: 'Back',
          icon: '↩',
          color: '#273549',
          action: { type: 'folder', folder_id: parentId },
          extensions: {},
        },
      ],
    };
    this.draft.layout.folders.push(folder);
    parent.buttons.push({
      id: id(),
      label: folder.label,
      icon: '▦',
      color: '#273549',
      action: { type: 'folder', folder_id: folder.id },
      extensions: { appearance: { cell } },
    });
    this.change();
    return folder;
  }
  renameFolder(folderId: string, label: string) {
    const folder = this.draft.layout.folders.find((folder) => folder.id === folderId);
    if (!folder) return;
    const previous = folder.label;
    folder.label = label;
    for (const folder of this.draft.layout.folders)
      for (const button of folder.buttons)
        if (
          button.action.type === 'folder' &&
          button.action.folder_id === folderId &&
          button.label === previous
        )
          button.label = label;
    this.change();
  }
  removeFolder(folderId: string) {
    if (this.draft.layout.folders.length <= 1)
      throw new Error('The deck needs at least one folder');
    if (!this.draft.layout.folders.some((folder) => folder.id === folderId)) return;
    this.recovery = clone(this.draft);
    this.freezeCells();
    for (const folder of this.draft.layout.folders)
      folder.buttons = folder.buttons.filter(
        (button) => !(button.action.type === 'folder' && button.action.folder_id === folderId),
      );
    this.draft.layout.folders = this.draft.layout.folders.filter(
      (folder) => folder.id !== folderId,
    );
    this.change();
    this.recoveryGeneration = this.generation;
  }
  get canUndoDeletion() {
    return this.recovery !== null && this.recoveryGeneration === this.generation;
  }
  undoDeletion() {
    if (!this.canUndoDeletion || !this.recovery) return;
    this.draft = clone(this.recovery);
    this.recovery = null;
    this.change();
  }
  duplicateButton(button: Button, folderId: string, label = `${button.label} copy`) {
    const folder = this.draft.layout.folders.find((folder) => folder.id === folderId);
    if (!folder) throw new Error('Destination folder is unavailable');
    this.freezeCells();
    const cell = placeButtons(
      folder.buttons,
      this.draft.layout.columns,
      this.draft.layout.rows,
    ).firstFree;
    const duplicate = clone(button);
    duplicate.id = id();
    duplicate.label = label;
    duplicate.extensions.appearance = {
      ...((duplicate.extensions.appearance as Record<string, unknown>) ?? {}),
      cell,
    };
    this.applyButton(duplicate, folderId, folderId);
    return duplicate;
  }
  restore(config: Config) {
    contract('Config', config);
    this.draft = clone(config);
    this.recovery = null;
    this.change();
  }
  persist(
    writer: (revision: number, config: Config) => Promise<ConfigResponse> = save,
  ): Promise<ConfigResponse> {
    if (this.pendingSave) return this.pendingSave;
    this.saving = true;
    this.failure = '';
    const generation = this.generation;
    this.pendingSave = (async () => {
      contract('Config', this.draft);
      const result = await writer(this.revision, clone(this.draft));
      this.saved(result, generation);
      return result;
    })()
      .catch((error: unknown) => {
        this.conflict = error instanceof ApiError && error.status === 409;
        this.failure = error instanceof Error ? error.message : String(error);
        throw error;
      })
      .finally(() => {
        this.saving = false;
        this.pendingSave = null;
      });
    return this.pendingSave;
  }
  saved(snapshot: ConfigResponse, generation = this.generation) {
    this.revision = snapshot.revision;
    this.persisted = clone(snapshot.config);
    this.conflict = false;
    if (generation === this.generation) {
      this.draft = clone(snapshot.config);
      this.dirty = false;
    }
    this.dirty = !sameContent(this.draft, this.persisted);
  }
}
