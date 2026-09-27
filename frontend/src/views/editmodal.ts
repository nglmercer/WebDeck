import { html, raw, type Html } from '../framework/html';
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
import { editorSaveButton, editorStyleBlock } from '../components/editor';
import { swapPreviewImage, updateButtonBackgroundColor, updateImageSize, type ButtonState } from './modalstyle';
import { svgSlot } from './svg';

/**
 * Port of editbutton_modal.jinja. `command_value` always resolves to
 * `button_settings` (the match loop is scope-dead) and `command_id`
 * renders "" — both verified against the live page.
 */
export function editButtonModal(
  ctx: BootContext,
  _folderId: string,
  _buttonId: number,
  editModalId: string,
  buttonSettings: JsonObject,
  _message: string
): Html {
  void _folderId;
  void _buttonId;
  void _message;
  const dark = ctx.dark_theme;
  const hasSettings = Object.keys(buttonSettings).length > 0;

  let fill = '';
  if ('color' in buttonSettings) {
    const color = asString(buttonSettings['color']);
    fill = color === 'invert' ? 'filter: invert(1)' : `fill:${color}; color:${color};`;
  }

  const buttonName = resolveButtonName(buttonSettings, undefined);
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const buttontextStyle =
    namesColor !== '' && namesColor.trim() !== '' ? `style="color:${namesColor};"` : '';

  const preview = hasSettings ? previewWithSettings(ctx, editModalId, buttonSettings, fill, buttonName, buttontextStyle) : previewEmpty(editModalId);

  const imageSize = asString(buttonSettings['image_size']);
  const defaultSize = imageSize !== '' && imageSize.trim() !== '' ? imageSize.trim().replace('%', '') : '75';

  const bgColor = asString(buttonSettings['background_color']);

  const nameValue = asString(buttonSettings['name']);
  const hasName = 'name' in buttonSettings && nameValue.trim() !== '';

  const devMode = asBool(get(ctx.config, 'settings', 'dev_mode'));

  return html`
<div class="editbutton-modal-container ${raw(dark)}" id="edit-modal-container-${editModalId}" edit_modal_ID="${editModalId}">
  <div class="editbutton-modal-content ${raw(dark)}">
    <div class="editbutton-modal-header bold edit-modal-container-${editModalId}">
      <h1 class="editbutton-modal"> ${text('configure_your_button')} </h1>
      <div class="editbutton-modal-close">
        <svg class="editbutton-config-modal ${raw(dark)}" xmlns="http://www.w3.org/2000/svg" width="19" height="19" fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16">
          <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z"/>
        </svg>
      </div>
    </div>
    <div class="editbutton-modal-main">
      <div class="config-container ${raw(dark)}">
        <form class="args-form" edit_modal_ID="${editModalId}" novalidate>
          <!--
          <div class="args-container ${raw(dark)}" edit_modal_ID="${editModalId}">
            future maj
          </div>
          -->
          ${editorStyleBlock({
            dark,
            id: editModalId,
            preview,
            defaultSize,
            backgroundColor: bgColor,
            buttonName,
            nameValue: hasName ? buttonName : '',
          })}
          ${
            devMode
              ? html`
          <div class="editorStyle-bar ${raw(dark)}"></div>
          <div class="arg_container" edit_modal_ID="${editModalId}">
            <label for="command_${editModalId}"> ${text('edit_command')} </label>
            <input class="${raw(dark)}" type="text" name="" id="command_${editModalId}" value="${rep(
              asString(buttonSettings['message']),
              '"',
              '&quot;'
            )}" />
          </div>`
              : raw('')
          }
          ${editorSaveButton(dark, editModalId)}
        </form>
      </div>
    </div>
  </div>
</div>`;
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

function previewWithSettings(
  ctx: BootContext,
  editModalId: string,
  buttonSettings: JsonObject,
  fill: string,
  buttonName: string,
  buttontextStyle: string
): Html {
  const image = asString(buttonSettings['image']);
  const bgBlock =
    asString(buttonSettings['background_color']) !== ''
      ? `background-color: ${asString(buttonSettings['background_color'])};\n                        box-shadow: ${asString(
          buttonSettings['background_color']
        )} 0 1px 3px 0;`
      : '';
  if (image === '') {
    return html`
                <button type="button" id="button-element_${editModalId}" class="wd_button" role="button"
                      style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;
                      ${raw(bgBlock)}
                      ">
                      <img id="button-image_${editModalId}" draggable="false" alt="" style="
                        width: ${String(112 * (50 / 100) + 3)}px;" />
                    </button>
                    <p class="buttontext" id="button-text-preview_${editModalId}" ${raw(buttontextStyle)}>
                      ${buttonName}
                    </p>`;
  }
  const imagelink = editImageLink(image);
  const imageSize = asString(buttonSettings['image_size']);
  const size = imageSize !== '' ? imageSize : '70%';
  const px = 112 * (parseInt(rep(size, '%', ''), 10) / 100) + 3;
  let media: Html;
  if (imagelink.endsWith('.svg')) {
    media = svgSlot(imagelink, ` id="button-image_${editModalId}" style="width:${px}px; height:${px}; ${fill}"`, '<svg');
  } else {
    media = html`<img id="button-image_${editModalId}" src="${imagelink}" draggable="false" alt="${imagelink}" onerror="this.remove()" style="
                            width: ${String(px)}px;
                            ${fill}"
                          />`;
  }
  return html`
                    <button type="button" id="button-element_${editModalId}" class="wd_button" role="button"
                    style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;
                    ${raw(bgBlock)}
                    ">
                      ${media}
                    </button>
                    <p class="buttontext" id="button-text-preview_${editModalId}" ${raw(buttontextStyle)}>
                      ${buttonName}
                    </p>`;
}

function previewEmpty(editModalId: string): Html {
  return html`
                    <button type="button" class="wd_button" role="button" style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;">
                      <img id="button-image_${editModalId}" draggable="false" alt="" style="
                        width: ${String(112 * (50 / 100) + 3)}px;" />
                    </button>
                    <p class="buttontext" id="button-text-preview_${editModalId}">
                      Button name
                    </p>`;
}

function editImageLink(image: string): string {
  if (image.startsWith('http')) return image;
  if (image.includes(':')) return 'static/img/' + (image.split('\\').pop() ?? image);
  if (image.startsWith('**uploaded/')) {
    return '.config/user_uploads/' + rep(image, '**uploaded/', '');
  }
  return 'static/img/' + image;
}

// --- Wire-up (per-modal <script> block) -------------------------------------

export interface EditModalState {
  button: ButtonState;
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

export function wireEditModal(
  ctx: BootContext,
  editModalId: string,
  buttonSettings: JsonObject,
  commandId: string
): void {
  const devMode = asBool(get(ctx.config, 'settings', 'dev_mode'));
  const button: ButtonState = { ...buttonSettings };
  button['name'] = byId(`button-text-preview_${editModalId}`).text()?.trim() ?? '';
  modalStates.set(editModalId, { button });

  const image = byId<HTMLElement>(`button-image_${editModalId}`).get(0) ?? null;
  const imageSizeSlider = byId<HTMLInputElement>(`image-size-slider_${editModalId}`).get(0) ?? null;
  const imageSizeValue = byId<HTMLInputElement>(`image-size-value_${editModalId}`).get(0) ?? null;
  if (image && imageSizeSlider && imageSizeValue) {
    updateImageSize(imageSizeSlider, imageSizeValue, image, button);
  }

  const buttonElement = byId<HTMLElement>(`button-element_${editModalId}`).get(0) ?? null;
  const bgInput = byId<HTMLInputElement>(`background-color-input_${editModalId}`).get(0) ?? null;
  const bgHex = byId<HTMLInputElement>(`background-color-hex_${editModalId}`).get(0) ?? null;
  if (buttonElement && bgInput && bgHex) {
    updateButtonBackgroundColor(buttonElement, bgInput, bgHex, button);
  }

  byId(`image-input_${editModalId}`).on('change', function () {
    console.log(`image-input_${editModalId} just got changed!`);
    const input = this as unknown as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append('file', file);

    void post('/upload_file', formData).then(
      () => {
        console.log('File downloaded successfully!');
        const swapped = swapPreviewImage(editModalId, input);
        if (!swapped) return;
        button['image_size'] = '70';
        updateImageSize(swapped.slider, swapped.value, swapped.image, button);
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
    setTimeout(function () {
      buttonCommand(editModalId, commandId, event);
    }, 1000);
  });
}

function buttonCommand(editModalID: string, command: string, event: Event): void {
  if (editModalID === 'NONE') {
    void command;
  } else {
    event.preventDefault();
    try {
      const inputs = q(`form[edit_modal_ID="${editModalID}"]`).find('input, select, textarea').toArray();
      const values = inputs
        .filter((input) => {
          if (q(input).parent().css('display') === 'none') {
            return false;
          }
          if (q(input).hasClass('choice')) {
            return false;
          }
          if (q(input).closest('.editorStyle, .webdeck_foldername_div').length > 0) {
            return false;
          }
          return true;
        })
        .map((input) => {
          if (q(input).is('select')) {
            const select = input as HTMLSelectElement;
            return select.options[select.selectedIndex]?.value ?? '';
          }
          const field = input as HTMLInputElement;
          const kind = q(field).prop('type');
          if (kind === 'radio' || kind === 'checkbox') {
            return q(field).prop('checked') === true ? String(q(field).val() ?? '') : '';
          } else if (kind === 'submit' || kind === 'button') {
            return '';
          } else {
            return String(q(field).val() ?? '');
          }
        })
        .filter((value) => value !== '');
      void (command + ' ' + values.join(' '));
    } catch {
      // Falls back to the untouched message (upstream String.raw template).
    }
  }

  console.log('buttonCommand received, from: edit');

  const state = modalStates.get(editModalID);
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
        alert(text('settings_save_success'));
        window.location.href = window.location.href + '?edit=true';
      } else {
        throw new Error(text('settings_save_error'));
      }
    })
    .catch(function (error: Error) {
      alert(error.message);
    });

  console.log(state?.button);
}
