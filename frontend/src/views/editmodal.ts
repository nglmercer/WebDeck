import { text } from '../framework/i18n';
import {
  asBool,
  asObject,
  asString,
  get,
  isObject,
  rep,
  type BootContext,
  type JsonObject,
  type JsonValue,
} from '../framework/types';
import { q, byId, post } from '../query';
import { emitAppEvent } from '../app/events';
import { hide_editbutton_modal } from '../app/modals';
import { refreshApp } from '../app/refresh';
import { iconFillStyle } from '../components/button-icons';
import { showAlert } from '../components/dialog';
import { wireKeyField } from '../components/keyfield';
import { previewImageLink, type PreviewData } from '../components/preview';
import { buildCommand, hasVisibleParams, registerShowArg, type ArgsPrefill } from './args';
import { resolveButtonCommand } from './argvalues';
import { wireFoldernameForm } from './addbutton';
import { swapPreviewImage, updateButtonBackgroundColor, updateImageSize, type ButtonState } from './modalstyle';
import { wireStudioPreview } from './studio-preview';
import { svgSlotId } from './svg';

/**
 * Edit-button modal data (editbutton_modal.jinja). The saved message
 * resolves back to its commands entry so the modal renders the same arg
 * form as the add modal (prefilled); unresolvable messages keep the
 * legacy form-less rendering. (Upstream's match loop is scope-dead —
 * `command_value` always fell back to `button_settings` and `command_id`
 * rendered "".) Markup out in `EditModal.svelte`.
 */
export interface EditModalArgs {
  category: string;
  command: string;
  subId: number;
  parentCommand: string;
  commandValue: JsonObject;
  prefill: ArgsPrefill;
}

export interface EditModalData {
  dark: string;
  modalId: string;
  args: EditModalArgs | null;
  /** Resolved command with at least one visible Parameters field. */
  hasParams: boolean;
  preview: PreviewData;
  defaultSize: string;
  backgroundColor: string;
  buttonName: string;
  nameValue: string;
  showDevbox: boolean;
  devboxValue: string;
}

export function editModalData(
  ctx: BootContext,
  editModalId: string,
  buttonSettings: JsonObject,
  message: string
): EditModalData {
  const dark = ctx.dark_theme;
  const resolved = resolveButtonCommand(ctx.commands, message);

  const fill = 'color' in buttonSettings ? iconFillStyle(asString(buttonSettings['color'])) : '';

  const buttonName = resolveButtonName(buttonSettings, undefined);
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const textStyle =
    namesColor !== '' && namesColor.trim() !== '' ? `color:${namesColor};` : null;

  const preview = editPreviewData(editModalId, buttonSettings, fill, buttonName, textStyle);

  const imageSize = asString(buttonSettings['image_size']);
  const defaultSize =
    imageSize !== '' && imageSize.trim() !== '' ? imageSize.trim().replace('%', '') : '75';

  const nameValue = asString(buttonSettings['name']);
  const hasName = 'name' in buttonSettings && nameValue.trim() !== '';

  return {
    dark,
    modalId: editModalId,
    args: resolved
      ? {
          category: resolved.category,
          command: resolved.command,
          subId: resolved.subId,
          parentCommand: resolved.parentCommand,
          commandValue: resolved.commandValue,
          prefill: resolved.prefill,
        }
      : null,
    hasParams: resolved ? hasVisibleParams(resolved.commandValue) : false,
    preview,
    defaultSize,
    backgroundColor: asString(buttonSettings['background_color']),
    buttonName,
    nameValue: hasName ? buttonName : '',
    showDevbox: asBool(get(ctx.config, 'settings', 'dev_mode')) && !resolved,
    // Parsed-HTML parity: the legacy template pre-replaced quotes, then
    // autoescaping doubled them back on parse (see grid hidden inputs).
    devboxValue: rep(asString(buttonSettings['message']), '"', '&quot;'),
  };
}

/** Button-name fallback chain (the `[language]` branch is dead upstream). */
function resolveButtonName(buttonSettings: JsonObject, commandStyleName: JsonValue | undefined): string {
  const name = buttonSettings['name'];
  if ('name' in buttonSettings && asString(name).trim() !== '') {
    if (typeof name === 'string') return name;
    if (isObject(name)) {
      const en = name['en'];
      if (typeof en === 'string' && en !== '') return en;
      const idx = (name as unknown as Record<string, JsonValue>)['0'];
      if (typeof idx === 'string' && idx !== '') return idx;
    }
    return '';
  }
  const styleName = commandStyleName;
  if (typeof styleName === 'string') return styleName;
  if (isObject(styleName)) {
    const en = styleName['en'];
    if (typeof en === 'string' && en !== '') return en;
    const idx = (styleName as unknown as Record<string, JsonValue>)['0'];
    if (typeof idx === 'string' && idx !== '') return idx;
  }
  return '';
}

/** Shared-preview data for the edit tile (never a usage overlay). */
export function editPreviewData(
  editModalId: string,
  buttonSettings: JsonObject,
  fill: string,
  buttonName: string,
  textStyle: string | null
): PreviewData {
  const base = 'overflow: hidden; overflow-y: hidden; max-height: 89.6px;';
  if (Object.keys(buttonSettings).length === 0) {
    return {
      id: editModalId,
      buttonId: false,
      buttonStyle: base,
      media: { kind: 'img', src: null, alt: '', removeOnError: false, widthPx: 112 * (50 / 100) + 3, fill: '' },
      usageFill: null,
      text: 'Button name',
      textStyle: null,
    };
  }
  const bg = asString(buttonSettings['background_color']);
  const buttonStyle = bg !== '' ? `${base} background-color: ${bg}; box-shadow: ${bg} 0 1px 3px 0;` : base;
  const image = asString(buttonSettings['image']);
  if (image === '') {
    return {
      id: editModalId,
      buttonId: true,
      buttonStyle,
      media: { kind: 'img', src: null, alt: '', removeOnError: false, widthPx: 112 * (50 / 100) + 3, fill: '' },
      usageFill: null,
      text: buttonName,
      textStyle,
    };
  }
  const imagelink = previewImageLink(image);
  const imageSize = asString(buttonSettings['image_size']);
  const size = imageSize !== '' ? imageSize : '70%';
  const px = 112 * (parseInt(rep(size, '%', ''), 10) / 100) + 3;
  return {
    id: editModalId,
    buttonId: true,
    buttonStyle,
    media: imagelink.endsWith('.svg')
      ? {
          kind: 'svg',
          slot: svgSlotId(
            imagelink,
            ` id="button-image_${editModalId}" style="width:${px}px; height:${px}; ${fill}"`,
            '<svg'
          ),
        }
      : { kind: 'img', src: imagelink, alt: imagelink, removeOnError: true, widthPx: px, fill },
    usageFill: null,
    text: buttonName,
    textStyle,
  };
}

// --- Wire-up (per-modal <script> block) -------------------------------------

export interface EditModalState {
  button: ButtonState;
  /** Resolved command id when the modal renders an arg form (message rebuild). */
  commandId?: string;
  /** Submit coalescing: a delayed save is already scheduled/in flight. */
  submitPending?: boolean;
}

const modalStates = new Map<string, EditModalState>();

export function editModalState(editModalId: string): EditModalState | undefined {
  return modalStates.get(editModalId);
}

export interface EditModalTarget {
  editModalId: string;
  folderId: string;
  buttonId: number;
  entry: JsonObject;
}

/** Iterate grid buttons exactly like the view (for post-render wiring). */
export function collectEditModals(ctx: BootContext): EditModalTarget[] {
  const out: EditModalTarget[] = [];
  const buttons = asObject(get(ctx.config, 'front', 'buttons'));
  Object.entries(buttons).forEach(([folderId, value], folderIndex) => {
    (Array.isArray(value) ? value : []).forEach((buttonConfig, buttonId) => {
      const entry = asObject(buttonConfig);
      const isVoid = 'VOID' in entry || Object.keys(entry).length === 0;
      if (!isVoid) {
        out.push({ editModalId: `e${folderIndex}X${buttonId}`, folderId, buttonId, entry });
      }
    });
  });
  return out;
}

export function wireEditModal(ctx: BootContext, editModalId: string, buttonSettings: JsonObject): void {
  const devMode = asBool(get(ctx.config, 'settings', 'dev_mode'));
  const button: ButtonState = { ...buttonSettings };
  button['name'] = byId(`button-text-preview_${editModalId}`).text()?.trim() ?? '';
  const resolved = resolveButtonCommand(ctx.commands, asString(buttonSettings['message']));
  const state: EditModalState = { button };
  if (resolved) state.commandId = resolved.commandId;
  modalStates.set(editModalId, state);

  registerShowArg(editModalId, 'edit_modal_ID');
  wireFoldernameForm(editModalId);
  wireKeyField(editModalId);
  wireStudioPreview(editModalId);

  const image = byId<HTMLElement>(`button-image_${editModalId}`).get(0) ?? null;
  const imageSizeSlider = byId<HTMLInputElement>(`image-size-slider_${editModalId}`).get(0) ?? null;
  if (image && imageSizeSlider) {
    updateImageSize(imageSizeSlider, image, button);
  }

  const buttonElement = byId<HTMLElement>(`button-element_${editModalId}`).get(0) ?? null;
  const bgInput = byId<HTMLInputElement>(`background-color-input_${editModalId}`).get(0) ?? null;
  const bgHex = byId<HTMLInputElement>(`background-color-hex_${editModalId}`).get(0) ?? null;
  if (buttonElement && bgInput && bgHex) {
    updateButtonBackgroundColor(buttonElement, bgInput, bgHex, button);
  }

  byId(`image-input_${editModalId}`).on('change', function () {
    const input = this as unknown as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append('file', file);

    void post('/upload_file', formData).then(
      () => {
        const swapped = swapPreviewImage(editModalId, input);
        if (!swapped) return;
        button['image_size'] = '70';
        updateImageSize(swapped.slider, swapped.image, button);
        button['image'] = '**uploaded/' + (input.files?.[0]?.name ?? '');
      },
      () => {
        console.error('Failed to download file.');
      }
    );
  });

  if (devMode) {
    byId(`command_${editModalId}`).on('input', function () {
      button['message'] = String(q(this).val() ?? '');
    });
  }

  const buttonText = byId<HTMLInputElement>(`button-text-input_${editModalId}`).get(0) ?? null;
  const buttonPreview = byId(`button-text-preview_${editModalId}`).get(0) ?? null;
  q(buttonText).on('input', function () {
    const textValue = String(q(buttonText).val() ?? '');
    if (buttonPreview) q(buttonPreview).text(textValue);
    button['name'] = textValue;
  });

  byId(`${editModalId}_submit`).on('click', function (event) {
    // The delayed save invites double-clicks (nothing happens for a
    // second): ignore while one is already pending.
    const pending = modalStates.get(editModalId);
    if (pending?.submitPending) return;
    if (pending) pending.submitPending = true;
    setTimeout(function () {
      buttonCommand(editModalId, event);
    }, 1000);
  });
}

function buttonCommand(editModalID: string, event: Event): void {
  if (editModalID !== 'NONE') {
    event.preventDefault();
  }

  const state = modalStates.get(editModalID);
  if (state && state.commandId !== undefined) {
    const container = q(`form[edit_modal_ID="${editModalID}"] .args-container`).get(0) ?? null;
    if (container) {
      state.button['message'] = buildCommand(state.commandId, container);
    }
  }

  const final_button = {
    location_Folder: rep(editModalID, 'e', '').split('X')[0],
    location_Id: rep(editModalID, 'e', '').split('X')[1],
    content: state?.button ?? {},
  };

  fetch('/save_single_button', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(final_button),
  })
    .then(function (response) {
      if (response.ok) {
        return response.json();
      } else {
        throw new Error(text('settings_save_error'));
      }
    })
    .then(function (response: { success?: boolean }) {
      if (response.success) {
        emitAppEvent('save:completed', { flow: 'single' });
        hide_editbutton_modal(editModalID);
        void refreshApp();
        void showAlert(text('settings_save_success'));
      } else {
        throw new Error(text('settings_save_error'));
      }
    })
    .catch(function (error: Error) {
      void showAlert(error.message);
    })
    .finally(function () {
      const pending = modalStates.get(editModalID);
      if (pending) pending.submitPending = false;
    });
}
