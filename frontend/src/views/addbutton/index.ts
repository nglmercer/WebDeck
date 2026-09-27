// Add-button barrel (split from addbutton.ts; './addbutton' import paths unchanged).

export type { AddModalContext } from './types';
export { addBrowserView, addModalChrome } from './browser';
export { addArgsModal } from './argsmodal';
export {
  collectAddModals,
  getCommand,
  wireAddModal,
  wireBrowserDropdowns,
  wireBrowserSearch,
  wireFoldernameForm,
} from './wireup';
