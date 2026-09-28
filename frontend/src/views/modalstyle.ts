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

/** Studio `%` badge next to the slider (best-effort; absent pre-redesign). */
function syncSizeBadge(sizeInput: HTMLInputElement, parsedValue: number): void {
  const badgeId = sizeInput.id.replace('image-size-value_', 'size-pct_');
  if (badgeId === sizeInput.id) return;
  const badge = document.getElementById(badgeId);
  if (badge) badge.textContent = `${parsedValue} %`;
  const metaId = sizeInput.id.replace('image-size-value_', 'meta-size_');
  const meta = document.getElementById(metaId);
  if (meta) meta.textContent = `${parsedValue} %`;
}

export function updateImageSize(
  imageSizeSlider: HTMLInputElement,
  imageSizeValue: HTMLInputElement,
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

    q(imageSizeValue).val(String(parsedValue));
    syncSizeBadge(imageSizeValue, parsedValue);
    button['image_size'] = `${parsedValue}%`;
  });

  q(imageSizeValue).on('input', function (event) {
    const newValue = String(q(imageSizeValue).val() ?? '');
    const parsedValue = parseInt(newValue);

    if (!isNaN(parsedValue)) {
      q(imageSizeSlider).val(String(parsedValue));

      const calculatedSize = 112 * (parsedValue / 100) + 3;
      q(imageElement).css('width', calculatedSize + 'px');

      if (q(imageElement).is('svg')) {
        q(imageElement).css('height', calculatedSize + 'px');
        svgSizeExpandos(imageElement, calculatedSize);
      }

      syncSizeBadge(imageSizeValue, parsedValue);
      button['image_size'] = `${parsedValue}%`;
    }

    event.preventDefault();
  });

  q(imageSizeValue).on('keypress', function (event) {
    const charCode = event.which ? event.which : event.keyCode;

    if (charCode < 48 || charCode > 57) {
      event.preventDefault();
    }
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
): { image: HTMLElement; slider: HTMLInputElement; value: HTMLInputElement } | null {
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
  const value = byId<HTMLInputElement>(`image-size-value_${modalId}`).get(0);
  if (!image || !slider || !value) return null;

  q(slider).val('70');
  q(value).val('70');
  syncSizeBadge(value, 70);
  q(image).css('width', '81.4');
  return { image, slider, value };
}
