import { normalizeHexValue } from '../legacy/colors';
import { q, byId } from '../query';

// Shared by add/edit button modals: the per-modal preview wiring
// (updateImageSize / updateButtonBackgroundColor), parameterized by id.

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

export function updateButtonBackgroundColor(
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
