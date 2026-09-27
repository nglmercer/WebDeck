import { q, byId } from '../query';
import { normalizeHexValue } from './colors';

// Port of static/js/color-setting.js. Runs after render.

function handleInput(input: HTMLInputElement | null, output: HTMLInputElement | null): void {
  if (!input || !output) return;
  q(input).on('input', function () {
    const hexValue = normalizeHexValue(String(q(input).val() ?? ''));
    q(input).val(hexValue);
    q(output).val(hexValue);
  });
}

export function initColorSetting(): void {
  handleInput(
    byId<HTMLInputElement>('names-color-input').get(0) ?? null,
    byId<HTMLInputElement>('names-color-hex').get(0) ?? null
  );
  handleInput(
    byId<HTMLInputElement>('buttons-color-input').get(0) ?? null,
    byId<HTMLInputElement>('buttons-color-hex').get(0) ?? null
  );
}
