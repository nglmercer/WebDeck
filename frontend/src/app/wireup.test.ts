import { describe, expect, it } from 'vitest';
import { configureFolders, navigateFolder, deckState } from '../features/deck/state.svelte';

describe('typed folder navigation', () => {
  it('uses raw names as data and ignores missing folders', () => {
    const name = "quotes\" and ' backticks` ${notCode}";
    configureFolders(['index', name]);
    navigateFolder(name);
    expect(deckState.activeFolder).toBe(name);
    navigateFolder('missing');
    expect(deckState.activeFolder).toBe(name);
    configureFolders(['index']);
    expect(deckState.activeFolder).toBe('index');
  });
});
