import { normalizeHexValue } from '../components/colors';
import { q, byId } from '../query';

// Shared by add/edit button modals: the per-modal preview wiring,
// field synchronization, and submit coalescing — parameterized by id.

export type ButtonState = Record<string, unknown>;

function svgSizeExpandos(imageElement: HTMLElement, calculatedSize: number): void {
  // 1:1 upstream quirk: expando (not attribute) assignment on <svg>.
  // qdom's typed .prop() cannot express expandos, so this stays native.
  (imageElement as unknown as Record<string, unknown>)['height'] = calculatedSize + 'px';
  (imageElement as unknown as Record<string, unknown>)['width'] = calculatedSize + 'px';
}

/**
 * Studio sidebar `Size` readout (best-effort; absent outside split layout).
 * The slider is the single size input — the live tile plus this readout
 * replace the old slider + number + badge triplet.
 */
function syncSizeMeta(slider: HTMLInputElement, parsedValue: number): void {
  const prefix = 'image-size-slider_';
  if (!slider.id.startsWith(prefix)) return;
  const meta = document.getElementById('meta-size_' + slider.id.slice(prefix.length));
  if (meta) meta.textContent = `${parsedValue} %`;
}

export function updateImageSize(
  imageSizeSlider: HTMLInputElement,
  imageElement: HTMLElement,
  button: ButtonState
): void {
  q(imageSizeSlider).on('input', function () {
    const imageSize = String(q(imageSizeSlider).val() ?? '');
    const parsedValue = parseInt(imageSize);

    const calculatedSize = 112 * (parsedValue / 100) + 3;
    q(imageElement).css('width', calculatedSize + 'px');

    if (q(imageElement).is('svg')) {
      q(imageElement).css('height', calculatedSize + 'px');
      svgSizeExpandos(imageElement, calculatedSize);
    }

    syncSizeMeta(imageSizeSlider, parsedValue);
    button['image_size'] = `${parsedValue}%`;
  });
}

function updateButtonBackgroundColor(
  buttonElement: HTMLElement,
  buttonBackgroundColorInput: HTMLInputElement,
  buttonBackgroundColorHex: HTMLInputElement,
  button: ButtonState
): void {
  q(buttonBackgroundColorInput).on('input', function () {
    const colorValue = String(q(buttonBackgroundColorInput).val() ?? '');
    const hexValue = normalizeHexValue(colorValue);

    q(buttonBackgroundColorInput).val(hexValue);
    q(buttonBackgroundColorHex).val(hexValue);
    button['background_color'] = hexValue;

    q(buttonElement).css({ backgroundColor: hexValue, boxShadow: '0 0 5px ' + hexValue });
  });

  q(buttonBackgroundColorHex).on('input', function () {
    const hexValue = String(q(buttonBackgroundColorHex).val() ?? '');
    const normalizedHexValue = normalizeHexValue(hexValue);

    q(buttonBackgroundColorInput).val(normalizedHexValue);
    q(buttonBackgroundColorHex).val(normalizedHexValue);
    button['background_color'] = normalizedHexValue;

    q(buttonElement).css({ backgroundColor: normalizedHexValue, boxShadow: '0 0 5px ' + normalizedHexValue });
  });
}

/** Image-size slider + background-color inputs shared by both modals. */
export function wirePreviewControls(modalId: string, button: ButtonState): void {
  const image = byId<HTMLElement>(`button-image_${modalId}`).get(0) ?? null;
  const imageSizeSlider = byId<HTMLInputElement>(`image-size-slider_${modalId}`).get(0) ?? null;
  if (image && imageSizeSlider) {
    updateImageSize(imageSizeSlider, image, button);
  }

  const buttonElement = byId<HTMLElement>(`button-element_${modalId}`).get(0) ?? null;
  const bgInput = byId<HTMLInputElement>(`background-color-input_${modalId}`).get(0) ?? null;
  const bgHex = byId<HTMLInputElement>(`background-color-hex_${modalId}`).get(0) ?? null;
  if (buttonElement && bgInput && bgHex) {
    updateButtonBackgroundColor(buttonElement, bgInput, bgHex, button);
  }
}

/** Button-name text input <-> preview <-> state sync shared by both modals. */
export function wireButtonNameSync(modalId: string, button: ButtonState): void {
  const buttonText = byId<HTMLInputElement>(`button-text-input_${modalId}`).get(0) ?? null;
  const buttonPreview = byId(`button-text-preview_${modalId}`).get(0) ?? null;
  q(buttonText).on('input', function () {
    const textValue = String(q(buttonText).val() ?? '');
    if (buttonPreview) q(buttonPreview).text(textValue);
    button['name'] = textValue;
  });
}

/** Dev-mode raw command input -> state sync shared by both modals. */
export function wireDevCommandSync(modalId: string, button: ButtonState): void {
  byId(`command_${modalId}`).on('input', function () {
    button['message'] = String(q(this).val() ?? '');
  });
}

/** Per-modal save state for submit coalescing (both flows match this shape). */
export interface ModalSubmitState {
  submitPending?: boolean;
}

/**
 * Guard a modal save against double-clicks: reject while a save is in
 * flight, otherwise mark pending and disable the submit control until
 * `endModalSubmit` runs (after success or failure).
 */
export function beginModalSubmit(
  state: ModalSubmitState | undefined,
  modalId: string
): boolean {
  if (!state || state.submitPending) return false;
  state.submitPending = true;
  const submit = byId<HTMLInputElement>(`${modalId}_submit`).get(0) ?? null;
  if (submit) submit.disabled = true;
  return true;
}

/** Release a coalesced save: clear the flag, re-enable the submit control. */
export function endModalSubmit(state: ModalSubmitState | undefined, modalId: string): void {
  if (state) state.submitPending = false;
  const submit = byId<HTMLInputElement>(`${modalId}_submit`).get(0) ?? null;
  if (submit) submit.disabled = false;
}

/** Shared image-upload swap used by both modals' image-input handlers. */
export function swapPreviewImage(
  modalId: string,
  input: HTMLInputElement
): { image: HTMLElement; slider: HTMLInputElement } | null {
  const element = byId<HTMLElement>(`button-image_${modalId}`).get(0);
  if (!element) return null;
  const fileName = input.files?.[0]?.name ?? '';
  const filePath = '.config/user_uploads/' + fileName;

  let image: HTMLElement | null;
  if (q(element).is('svg')) {
    q(element).replaceWith(
      q<HTMLImageElement>('<img>')
        .attr('id', `button-image_${modalId}`)
        .prop('draggable', false)
        .prop('src', filePath)
        .css('width', '87px')
    );
    image = byId<HTMLElement>(`button-image_${modalId}`).get(0) ?? null;
  } else {
    q(element).attr('src', filePath);
    image = byId<HTMLElement>(`button-image_${modalId}`).get(0) ?? null;
  }

  const slider = byId<HTMLInputElement>(`image-size-slider_${modalId}`).get(0);
  if (!image || !slider) return null;

  q(slider).val('70');
  syncSizeMeta(slider, 70);
  q(image).css('width', '81.4');
  return { image, slider };
}
