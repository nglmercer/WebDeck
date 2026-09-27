import { normalizeHexValue } from './colors';

// Port of static/js/color-setting.js. Runs after render.

function handleInput(input: HTMLInputElement | null, output: HTMLInputElement | null): void {
  if (!input || !output) return;
  input.addEventListener('input', function () {
    const hexValue = normalizeHexValue(input.value);
    input.value = hexValue;
    output.value = hexValue;
  });
}

export function initColorSetting(): void {
  handleInput(
    document.getElementById('names-color-input') as HTMLInputElement | null,
    document.getElementById('names-color-hex') as HTMLInputElement | null
  );
  handleInput(
    document.getElementById('buttons-color-input') as HTMLInputElement | null,
    document.getElementById('buttons-color-hex') as HTMLInputElement | null
  );
}
