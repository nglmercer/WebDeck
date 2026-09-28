import { mount } from 'svelte';
import KeyFieldView from './KeyFieldView.svelte';

/**
 * Key-selector field for `input key` args (e.g. Text / Press a key).
 *
 * One collected text input (serializes like a plain text arg) plus two
 * auxiliary controls that never serialize: a capture button that records
 * the next physical keypress, and a `searchDropdown` over the named
 * keys the backend `/key` handler accepts (see `map_key` in
 * `src/app/buttons/commands.rs`; its search box carries `key-aux` so
 * `collectArgValues` skips it).
 *
 * The element is a thin shell: rendering + interaction state live in the
 * Svelte interior (`KeyFieldView.svelte`), mounted here. Connection
 * auto-wires everything; `wireKeyField` stays as the id-based entry
 * point (no-op when the modal has no key field).
 */

/** Canonical named keys, in backend `map_key` order. */
export const NAMED_KEYS = [
  'ctrl',
  'rctrl',
  'shift',
  'rshift',
  'alt',
  'enter',
  'esc',
  'tab',
  'space',
  'backspace',
  'delete',
  'insert',
  'home',
  'end',
  'pageup',
  'pagedown',
  'up',
  'down',
  'left',
  'right',
  'win',
  'rwin',
  'capslock',
  'numlock',
  'scrolllock',
  'printscreen',
  'pause',
  'volumemute',
  'volumeup',
  'volumedown',
  'playpause',
  'prevtrack',
  'nexttrack',
  'mediastop',
  'f1',
  'f2',
  'f3',
  'f4',
  'f5',
  'f6',
  'f7',
  'f8',
  'f9',
  'f10',
  'f11',
  'f12',
];

export class KeyField extends HTMLElement {
  private mounted = false;

  connectedCallback(): void {
    if (this.mounted) return;
    this.mounted = true;
    mount(KeyFieldView, {
      target: this,
      props: {
        fieldId: this.getAttribute('field-id') ?? '',
        dark: this.getAttribute('dark') ?? '',
        initialValue: this.getAttribute('value') ?? '',
      },
    });
    // NOTE: no flushSync here (same nested-mount rule as search-dropdown):
    // this element upgrades inside modal subtrees sharing the outer
    // batch; search wiring runs in the interior effect, post-mount.
  }
}

export function defineKeyField(): void {
  if (!customElements.get('key-field')) {
    customElements.define('key-field', KeyField);
  }
}

defineKeyField();

const CAPTURED_KEY_MAP: Record<string, string> = {
  ' ': 'space',
  Enter: 'enter',
  Tab: 'tab',
  Backspace: 'backspace',
  Delete: 'delete',
  Insert: 'insert',
  Home: 'home',
  End: 'end',
  PageUp: 'pageup',
  PageDown: 'pagedown',
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
  Meta: 'win',
  OS: 'win',
  Alt: 'alt',
  Control: 'ctrl',
  Shift: 'shift',
  CapsLock: 'capslock',
  NumLock: 'numlock',
  ScrollLock: 'scrolllock',
  PrintScreen: 'printscreen',
  Pause: 'pause',
  AudioVolumeMute: 'volumemute',
  AudioVolumeUp: 'volumeup',
  AudioVolumeDown: 'volumedown',
  MediaPlayPause: 'playpause',
  MediaTrackNext: 'nexttrack',
  MediaTrackPrevious: 'prevtrack',
  MediaStop: 'mediastop',
};

/**
 * Normalize a captured `KeyboardEvent.key` to a backend key name.
 * Returns `null` for Escape (cancel the capture; pick `esc` from the list).
 */
export function normalizeCapturedKey(key: string): string | null {
  if (key === 'Escape') return null;
  const mapped = CAPTURED_KEY_MAP[key];
  if (mapped !== undefined) return mapped;
  if (/^F\d{1,2}$/.test(key)) return key.toLowerCase();
  if (key.length === 1) return key;
  return key.toLowerCase();
}

/** Wire one modal's key field; no-op when the modal has none. */
export function wireKeyField(modalId: string): void {
  const el = document.getElementById(`key-field_${modalId}`);
  if (!(el instanceof KeyField)) return;
  // Connection auto-wires (capture UI + search list); nothing else needed.
}
