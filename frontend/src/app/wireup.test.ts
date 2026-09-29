import { describe, expect, it } from 'vitest';
import { deleteFolder } from './editor/void';
import { installGlobals } from './wireup';

describe('installGlobals', () => {
  it('installs window.deleteFolder (folder-tab string onclick handler)', () => {
    // Regression: the FolderDeleteIcon `deleteFolder('…')` handler, the
    // globals.d.ts declaration, and the void.ts implementation existed,
    // but nothing assigned window.deleteFolder — every click threw.
    installGlobals();
    expect(window.deleteFolder).toBe(deleteFolder);
  });

  it('installs the other inline-handler globals', () => {
    installGlobals();
    expect(typeof window.folder).toBe('function');
    expect(typeof window.togglePasswordVisibility).toBe('function');
    expect(typeof window.send_data).toBe('function');
  });
});
