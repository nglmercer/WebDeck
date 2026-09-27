// Shared add-modal context (extracted from addbutton.ts).

import type { JsonObject } from '../../framework/types';

export interface AddModalContext {
  argModalId: string;
  category: string;
  command: string;
  parentCommand: string;
  subId: number;
  commandValue: JsonObject;
  commandId: string;
  buttonTitle: string;
}
