// Shared by the color/background settings (normalizeHexValue lived in the
// index inline script; behavior kept identical).

/** Exact port of the per-modal `normalizeHexValue` (prepend `#` only). */
export function normalizeHexValue(value: string): string {
  if (value.charAt(0) !== '#') {
    value = '#' + value;
  }
  return value;
}

export function calculateBrightness(color: string): number {
  let r: number;
  let g: number;
  let b: number;
  if (color.startsWith('#')) {
    r = parseInt(color.substr(1, 2), 16);
    g = parseInt(color.substr(3, 2), 16);
    b = parseInt(color.substr(5, 2), 16);
  } else if (color.startsWith('rgb(')) {
    const rgbValues = color.substring(4, color.length - 1).split(',');
    r = parseInt((rgbValues[0] ?? '').trim());
    g = parseInt((rgbValues[1] ?? '').trim());
    b = parseInt((rgbValues[2] ?? '').trim());
  } else {
    console.error('Invalid color format');
    return NaN;
  }
  return (r * 299 + g * 587 + b * 114) / 1000;
}
