// Shared mutable editor UI state (was module-level `let`s in editor.ts).
//
// A single object (rather than re-exported `let`s) so every editor module
// observes the same assignments.

export const editorUiState: {
  ifModif: number;
  swapMode: number;
  savedOnClicks: Record<string, string | null>;
  swapChanges: string[];
  swapUNChanges: string[];
  swapFirstBtn: string | 0;
  swapSecondBtn: string | 0;
  isMouseOverOpenFolder: boolean;
} = {
  ifModif: 0,
  swapMode: 0,
  savedOnClicks: {},
  swapChanges: [],
  swapUNChanges: [],
  swapFirstBtn: 0,
  swapSecondBtn: 0,
  isMouseOverOpenFolder: false,
};

export function isSwapMode(): boolean {
  return editorUiState.swapMode === 1;
}
