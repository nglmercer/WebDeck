// Editor mode barrel (split from editor.ts; './editor' import paths unchanged).

export { isSwapMode } from './state';
export { swapEditorButtonFunction, undoSwap, undoUNSwap } from './swap';
export { SaveExitEditor, toggleEditorMode } from './mode';
export { reloadEditorEvents, wireEditorChrome } from './events';
