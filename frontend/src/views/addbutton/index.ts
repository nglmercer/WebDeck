// Add-button barrel (split from addbutton.ts; './addbutton' import paths unchanged).

export type { AddModalContext } from './types';
export { addBrowserData, type AddBrowserData, type BrowserCategory, type BrowserItem, type BrowserLeaf, type BrowserBranch } from './browser';
export { addArgsData, addButtonName, type AddArgsData } from './argsmodal';
export { addPreviewData } from './preview';
export {
  collectAddModals,
  getCommand,
  wireAddModal,
  wireBrowserDropdowns,
  wireBrowserSearch,
  wireFoldernameForm,
} from './wireup';
