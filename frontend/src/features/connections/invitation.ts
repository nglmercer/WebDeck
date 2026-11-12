import { writable } from 'svelte/store';

/** Presentation owned by ConnectionHub; approvals share its dialog, not its polling. */
export type InvitationPresentation = {
  url: string;
  qr: string;
  progress: string;
  expired: boolean;
  working: boolean;
  tutorial: boolean;
  close: () => void;
  refresh: () => void;
  copy: () => void;
};
export const connectionInvitation = writable<InvitationPresentation | null>(null);
