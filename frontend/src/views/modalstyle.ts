import { normalizeHexValue } from '../legacy/colors';

// Shared by add/edit button modals: the per-modal preview wiring
// (updateImageSize / updateButtonBackgroundColor), parameterized by id.

export type ButtonState = Record<string, unknown>;

export function updateImageSize(
  imageSizeSlider: HTMLInputElement,
  imageSizeValue: HTMLInputElement,
  imageElement: HTMLElement,
  button: ButtonState
): void {
  imageSizeSlider.addEventListener('input', function () {
    const imageSize = imageSizeSlider.value;
    const parsedValue = parseInt(imageSize);

    const calculatedSize = 112 * (parsedValue / 100) + 3;
    imageElement.style.width = calculatedSize + 'px';

    if (imageElement.tagName === 'svg') {
      imageElement.style.height = calculatedSize + 'px';
      (imageElement as unknown as Record<string, unknown>)['height'] = calculatedSize + 'px';
      (imageElement as unknown as Record<string, unknown>)['width'] = calculatedSize + 'px';
    }

    imageSizeValue.value = String(parsedValue);
    button['image_size'] = `${parsedValue}%`;
  });

  imageSizeValue.addEventListener('input', function (event) {
    const newValue = imageSizeValue.value;
    const parsedValue = parseInt(newValue);

    if (!isNaN(parsedValue)) {
      imageSizeSlider.value = String(parsedValue);

      const calculatedSize = 112 * (parsedValue / 100) + 3;
      imageElement.style.width = calculatedSize + 'px';

      if (imageElement.tagName === 'svg') {
        imageElement.style.height = calculatedSize + 'px';
        (imageElement as unknown as Record<string, unknown>)['height'] = calculatedSize + 'px';
        (imageElement as unknown as Record<string, unknown>)['width'] = calculatedSize + 'px';
      }

      button['image_size'] = `${parsedValue}%`;
    }

    event.preventDefault();
  });

  imageSizeValue.addEventListener('keypress', function (event) {
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
  buttonBackgroundColorInput.addEventListener('input', function () {
    const colorValue = buttonBackgroundColorInput.value;
    const hexValue = normalizeHexValue(colorValue);

    buttonBackgroundColorInput.value = hexValue;
    buttonBackgroundColorHex.value = hexValue;
    button['background_color'] = hexValue;

    buttonElement.style.backgroundColor = hexValue;
    buttonElement.style.boxShadow = '0 0 5px ' + hexValue;
  });

  buttonBackgroundColorHex.addEventListener('input', function () {
    const hexValue = buttonBackgroundColorHex.value;
    const normalizedHexValue = normalizeHexValue(hexValue);

    buttonBackgroundColorInput.value = normalizedHexValue;
    buttonBackgroundColorHex.value = normalizedHexValue;
    button['background_color'] = normalizedHexValue;

    buttonElement.style.backgroundColor = normalizedHexValue;
    buttonElement.style.boxShadow = '0 0 5px ' + normalizedHexValue;
  });
}

/** Shared image-upload swap used by both modals' image-input handlers. */
export function swapPreviewImage(
  modalId: string,
  input: HTMLInputElement
): { image: HTMLElement; slider: HTMLInputElement; value: HTMLInputElement } | null {
  const element = document.getElementById(`button-image_${modalId}`);
  if (!element) return null;
  const fileName = input.files?.[0]?.name ?? '';
  const filePath = '.config/user_uploads/' + fileName;

  let image: HTMLElement | null;
  if (element.tagName.toLowerCase() === 'svg') {
    const imgElement = document.createElement('img');
    imgElement.id = `button-image_${modalId}`;
    imgElement.draggable = false;
    imgElement.src = filePath;
    imgElement.style.width = '87px';
    element.parentNode?.replaceChild(imgElement, element);
    image = document.getElementById(`button-image_${modalId}`);
  } else {
    (element as HTMLImageElement).src = filePath;
    image = document.getElementById(`button-image_${modalId}`);
  }

  const slider = document.getElementById(`image-size-slider_${modalId}`) as HTMLInputElement | null;
  const value = document.getElementById(`image-size-value_${modalId}`) as HTMLInputElement | null;
  if (!image || !slider || !value) return null;

  slider.value = '70';
  value.value = '70';
  image.style.width = '81.4';
  return { image, slider, value };
}
