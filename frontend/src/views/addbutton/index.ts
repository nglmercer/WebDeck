// Add-button barrel (split from addbutton.ts; './addbutton' import paths unchanged).

export type { AddModalContext } from './types';
export {
  collectAddModals,
  getCommand,
  wireAddModal,
  wireBrowserDropdowns,
  wireBrowserSearch,
  wireFoldernameForm,
} from './wireup';
