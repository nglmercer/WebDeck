import { describe, it, expect } from 'vitest';
import { Editor, clone } from './editor.svelte';
import { ButtonDraft } from './button-draft.svelte';
import type { Config } from '../../lib/contracts';
import initial from '../../../../webdeck/config_default.json';
import { placeButtons } from '../deck/deck';
const snapshot = () => ({ api_version: 2 as const, revision: 1, config: clone(initial) as Config });
describe('editor save generations', () => {
  it('retains edits made while a save is pending and advances the revision', () => {
    const editor = new Editor(snapshot());
    editor.draft.layout.rows = 4;
    editor.change();
    const generation = editor.generation;
    const saved = clone(editor.draft);
    editor.draft.layout.rows = 5;
    editor.change();
    editor.saved({ api_version: 2, revision: 2, config: saved }, generation);
    expect(editor.revision).toBe(2);
    expect(editor.draft.layout.rows).toBe(5);
    expect(editor.dirty).toBe(true);
  });
  it('marks the matching generation clean', () => {
    const editor = new Editor(snapshot());
    editor.change();
    editor.saved({ ...snapshot(), revision: 2 }, editor.generation);
    expect(editor.dirty).toBe(false);
  });
});

it('applies placement swaps only at commit and preserves unknown extensions', () => {
  const editor = new Editor(snapshot());
  editor.freezeCells();
  const folder = editor.draft.layout.folders[0]!;
  const original = folder.buttons[0]!;
  const neighbor = folder.buttons[1]!;
  original.extensions.custom = { retained: true };
  const draft = clone(original);
  (draft.extensions.appearance as Record<string, unknown>).cell = 1;
  expect((neighbor.extensions.appearance as Record<string, unknown>).cell).toBe(1);
  editor.applyButton(draft, folder.id, folder.id);
  expect((folder.buttons[1]!.extensions.appearance as Record<string, unknown>).cell).toBe(0);
  expect(folder.buttons[0]!.extensions.custom).toEqual({ retained: true });
});

it('deleting a folder removes incoming links and never removes the last folder', () => {
  const editor = new Editor(snapshot());
  const parent = editor.draft.layout.folders[0]!;
  const child = editor.createFolder(parent.id);
  expect(
    parent.buttons.some(
      (button) => button.action.type === 'folder' && button.action.folder_id === child.id,
    ),
  ).toBe(true);
  editor.removeFolder(child.id);
  expect(
    parent.buttons.some(
      (button) => button.action.type === 'folder' && button.action.folder_id === child.id,
    ),
  ).toBe(false);
  for (const folder of [...editor.draft.layout.folders].slice(1)) editor.removeFolder(folder.id);
  expect(() => editor.removeFolder(parent.id)).toThrow('at least one folder');
});

it('owns save serialization and preserves changes made after the submitted snapshot', async () => {
  const editor = new Editor(snapshot());
  editor.draft.layout.rows = 4;
  editor.change();
  let finish!: (result: ReturnType<typeof snapshot>) => void;
  let sent = clone(editor.draft);
  const writer = (_revision: number, config: Config) => {
    sent = config;
    return new Promise<ReturnType<typeof snapshot>>((resolve) => (finish = resolve));
  };
  const pending = editor.persist(writer);
  expect(editor.persist(writer)).toBe(pending);
  editor.draft.layout.rows = 5;
  editor.change();
  expect(sent.layout.rows).toBe(4);
  finish({ api_version: 2, revision: 2, config: sent });
  await pending;
  expect(editor.draft.layout.rows).toBe(5);
  expect(editor.dirty).toBe(true);
  expect(editor.saving).toBe(false);
});

it('duplicates with a new ID and undoes deletion without losing extension data', () => {
  const editor = new Editor(snapshot());
  const folder = editor.draft.layout.folders[0]!;
  const original = folder.buttons[0]!;
  original.extensions.vendor = { retained: true };
  const duplicate = editor.duplicateButton(original, folder.id);
  expect(duplicate.id).not.toBe(original.id);
  expect(duplicate.action).toEqual(original.action);
  expect(duplicate.extensions.vendor).toEqual({ retained: true });
  editor.removeButton(duplicate.id);
  expect(editor.canUndoDeletion).toBe(true);
  editor.undoDeletion();
  expect(editor.draft.layout.folders[0]!.buttons.some((button) => button.id === duplicate.id)).toBe(
    true,
  );
  expect(editor.canUndoDeletion).toBe(false);
});

it('undo compares against the latest persisted snapshot, including an in-flight deletion save', async () => {
  const editor = new Editor(snapshot());
  const button = editor.draft.layout.folders[0]!.buttons[0]!;
  editor.removeButton(button.id);
  editor.undoDeletion();
  expect(editor.dirty).toBe(false);
  editor.removeButton(button.id);
  let finish!: (value: ReturnType<typeof snapshot>) => void;
  const deleted = clone(editor.draft);
  const pending = editor.persist(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  editor.undoDeletion();
  expect(editor.dirty).toBe(false);
  finish({ api_version: 2, revision: 2, config: deleted });
  await pending;
  expect(editor.dirty).toBe(true);
  expect(editor.draft.layout.folders[0]!.buttons.some((item) => item.id === button.id)).toBe(true);
});

it('reverted edits and reordered extension object keys are clean while array order matters', () => {
  const initial = snapshot();
  initial.config.extensions.vendor = { first: 1, second: [2, 3] };
  const editor = new Editor(initial);
  editor.setDimensions('rows', initial.config.layout.rows + 1);
  expect(editor.dirty).toBe(true);
  editor.setDimensions('rows', initial.config.layout.rows);
  expect(editor.dirty).toBe(false);
  editor.draft.extensions.vendor = { second: [2, 3], first: 1 };
  editor.change();
  expect(editor.dirty).toBe(false);
  editor.draft.extensions.vendor = { first: 1, second: [3, 2] };
  editor.change();
  expect(editor.dirty).toBe(true);
});

it('opens and duplicates sparse fixed buttons without allocating the empty grid', () => {
  const initial = snapshot();
  initial.config.layout.folders[0]!.buttons[0]!.extensions.appearance = { cell: 2 ** 40 };
  const editor = new Editor(initial);
  const folder = editor.draft.layout.folders[0]!;
  const original = folder.buttons[0]!;
  const draft = new ButtonDraft(
    () => editor,
    () => folder,
  );
  draft.open(original);
  expect((draft.button!.extensions.appearance as Record<string, unknown>).cell).toBe(2 ** 40);
  expect(draft.dirty).toBe(false);
  const copy = editor.duplicateButton(original, folder.id);
  expect((copy.extensions.appearance as Record<string, unknown>).cell).toBeLessThan(12);
  editor.createFolder(folder.id);
  expect((original.extensions.appearance as Record<string, unknown>).cell).toBe(2 ** 40);
});

it('stages resizing and preserves IDs, order and extensions on commit', () => {
  const editor = new Editor(snapshot());
  editor.freezeCells();
  const folder = editor.draft.layout.folders[0]!;
  folder.buttons[0]!.extensions.vendor = { nested: ['keep', 42] };
  const before = clone(editor.draft);
  const draft = new ButtonDraft(
    () => editor,
    () => folder,
  );
  draft.open(folder.buttons[0]!);
  Object.assign(draft.button!.extensions.appearance!, { columns: 2, rows: 2 });
  expect(editor.draft).toEqual(before);
  draft.commit();
  expect(folder.buttons.map((button) => button.id)).toEqual(
    before.layout.folders[0]!.buttons.map((button) => button.id),
  );
  expect(folder.buttons[0]!.extensions.vendor).toEqual({ nested: ['keep', 42] });
  expect(folder.buttons[0]!.extensions.appearance).toMatchObject({ cell: 0, columns: 2, rows: 2 });
  const persisted = clone(folder.buttons);
  const placement = placeButtons(folder.buttons, editor.draft.layout.columns, 1);
  expect(placement.placed.size).toBe(folder.buttons.length);
  expect(placement.count).toBeGreaterThan(editor.draft.layout.columns);
  expect(folder.buttons).toEqual(persisted);
});

it('cannot delete the root deck even when other folders exist', () => {
  const editor = new Editor(snapshot());
  const before = clone(editor.draft);
  expect(() => editor.removeFolder(before.layout.folders[0]!.id)).toThrow('root deck');
  expect(editor.draft).toEqual(before);
  expect(editor.dirty).toBe(false);
  expect(editor.canUndoDeletion).toBe(false);
});
