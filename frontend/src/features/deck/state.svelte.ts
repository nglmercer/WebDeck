/** Svelte owns folder visibility; names are data, never JavaScript source. */
export const deckState = $state({ activeFolder: '', folders: [] as string[] });
export function configureFolders(folders: string[]): void {
  deckState.folders = [...folders];
  if (!folders.includes(deckState.activeFolder)) deckState.activeFolder = folders[0] ?? '';
}
export function navigateFolder(folder: string): void {
  if (deckState.folders.includes(folder)) deckState.activeFolder = folder;
}
