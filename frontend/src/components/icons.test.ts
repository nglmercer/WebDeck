import { describe, expect, it } from 'vitest';
import { raw } from '../framework/html';
import {
  addPlusIcon,
  deleteXIcon,
  editPencilIcon,
  folderDeleteIcon,
  modalCloseIcon,
  trashIcon,
} from './icons';

describe('icons', () => {
  it('renders the shared modal close glyph with the duplicated class quirk', () => {
    expect(modalCloseIcon('config-modal', raw('')).value).toBe(
      '<svg class="config-modal " xmlns="http://www.w3.org/2000/svg" width="19" height="19" ' +
        'fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16">' +
        '<path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 ' +
        '2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 ' +
        '0 0 0-.708-.708L8 7.293 5.354 4.646z"/></svg>'
    );
  });

  it('passes the theme suffix through unescaped', () => {
    expect(modalCloseIcon('config-modal', raw('dark-theme')).value).toContain(
      'class="config-modal dark-theme"'
    );
  });

  it('renders the add-slot plus glyph byte-identical to the inline version', () => {
    expect(addPlusIcon().value).toBe(
      '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24">' +
        '<path d="M12 4v16m8-8H4" stroke="currentColor" stroke-width="2" ' +
        'stroke-linecap="round" stroke-linejoin="round"/></svg>'
    );
  });

  it('renders the tile badge glyphs with currentColor strokes', () => {
    const pencil = editPencilIcon().value;
    expect(pencil).toContain('<svg width="16px" height="16px" viewBox="0 0 24 24" fill="none"');
    expect(pencil).toContain('id="Edit / Edit_Pencil_01"');
    expect(pencil).toContain('stroke="currentColor"');
    expect(pencil).toContain('M12 8.00012L4 16.0001V20.0001');

    const x = deleteXIcon().value;
    expect(x).toContain('<svg fill="currentColor"');
    expect(x).toContain('viewBox="0 0 32 32"');
    expect(x).toContain('M18.8,16l5.5-5.5c0.8-0.8,0.8-2,0-2.8');
  });

  it('keeps the inline double-escaping for folder ids', () => {
    // Callers pre-replace '"' with '&quot;'; the helper escapes again,
    // exactly like the inline template did.
    expect(folderDeleteIcon('a&quot;b').value).toContain("deleteFolder('a&amp;quot;b')");
    expect(folderDeleteIcon('index').value).toContain('class="bi bi-x-circle"');
    expect(folderDeleteIcon('index').value).toContain('M11.742 4.258a1 1 0 0 0-1.414 0');
  });

  it('escapes the trash title like the inline template did', () => {
    const out = trashIcon('<x>').value;
    expect(out).toContain('<title> &lt;x&gt; </title>');
    expect(out).toContain('class="bi bi-trash"');
    expect(out).toContain('M5.5 5.5A.5.5 0 0 1 6 6v6');
  });
});
