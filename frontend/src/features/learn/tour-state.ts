import { writable } from 'svelte/store';

/** Shared with dialogs so Driver.js stays in the same visual and focus layer. */
export const tutorialActive = writable(false);
