import { html, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import { byId, q } from '../query';
import { SearchDropdown, searchDropdown, wireSearchDropdown } from './search-dropdown';

/**
 * Key-selector field for `input key` args (e.g. Text / Press a key).
 *
 * One collected text input (serializes like a plain text arg) plus two
 * auxiliary controls that never serialize: a capture button that records
 * the next physical keypress, and a {@link searchDropdown} over the named
 * keys the backend `/key` handler accepts (see `map_key` in
 * `src/app/buttons/commands.rs`; its search box carries `key-aux` so
 * `collectArgValues` skips it).
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

export interface KeyFieldOptions {
  dark: string;
  /** Modal id suffix (unique per modal). */
  id: string;
  value: string;
}

export function keyField(o: KeyFieldOptions): Html {
  return html`<div class="key-field">
    <div class="key-field-row">
      <input class="${raw(o.dark)}" type="text" name="" size="10" id="key-input_${o.id}"${
        o.value !== '' ? html` value="${o.value}"` : raw('')
      } />
      <button type="button" class="key-capture ${raw(o.dark)}" id="key-capture_${o.id}">${text('key_capture')}</button>
    </div>
    ${searchDropdown({
      dark: o.dark,
      id: `key-list_${o.id}`,
      placeholder: text('key_search_keys'),
      inputClass: 'key-aux',
    })}
  </div>`;
}

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
  const input = byId<HTMLInputElement>(`key-input_${modalId}`).get(0) ?? null;
  const captureBtn = byId<HTMLButtonElement>(`key-capture_${modalId}`).get(0) ?? null;
  const list = byId(`key-list_${modalId}`).get(0) ?? null;
  if (!input || !captureBtn || !(list instanceof SearchDropdown)) return;

  wireSearchDropdown(`key-list_${modalId}`, NAMED_KEYS, (value) => {
    q(input).val(value);
  });

  const idleLabel = captureBtn.textContent ?? '';
  let armed = false;
  const disarm = (): void => {
    armed = false;
    q(captureBtn).text(idleLabel);
    q(captureBtn).removeClass('arming');
    document.removeEventListener('keydown', onKeydown, true);
  };
  const onKeydown = (event: Event): void => {
    const e = event as KeyboardEvent;
    e.preventDefault();
    e.stopPropagation();
    const name = normalizeCapturedKey(e.key);
    disarm();
    if (name !== null) q(input).val(name);
  };
  q(captureBtn).on('click', function () {
    if (armed) {
      disarm();
      return;
    }
    armed = true;
    q(captureBtn).text(text('key_capture_prompt'));
    q(captureBtn).addClass('arming');
    document.addEventListener('keydown', onKeydown, true);
  });
}
