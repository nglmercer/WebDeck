export type AppearanceOwner = { extensions: Record<string, unknown> };
export const appearanceBounds = {
  button_height: [24, 800],
  gap: [0, 100],
  radius: [0, 100],
  icon_size: [0, 200],
  columns: [1, 128],
  rows: [1, 128],
} as const;
export function appearanceOf(owner: AppearanceOwner | undefined): Record<string, unknown> {
  const value = owner?.extensions.appearance;
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
export function setAppearance(owner: AppearanceOwner, key: string, value: unknown) {
  if (key in appearanceBounds) {
    const [min, max] = appearanceBounds[key as keyof typeof appearanceBounds];
    if (
      typeof value !== 'number' ||
      !Number.isFinite(value) ||
      value < min ||
      value > max ||
      (['columns', 'rows'].includes(key) && !Number.isInteger(value))
    )
      throw new Error(`Invalid appearance value: ${key}`);
  }
  owner.extensions.appearance = { ...appearanceOf(owner), [key]: value };
}
export function tileColors(color: string): {
  background: string;
  foreground: string;
  contrast: number | null;
} {
  let hex = color.replace(/^#/, '');
  if (hex.length === 3)
    hex = hex
      .split('')
      .map((part) => part + part)
      .join('');
  if (!/^[a-f\d]{6}$/i.test(hex))
    return { background: color, foreground: 'var(--text)', contrast: null };
  const channels = [0, 2, 4].map((offset) => {
    const channel = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  const luminance = channels[0]! * 0.2126 + channels[1]! * 0.7152 + channels[2]! * 0.0722;
  const white = 1.05 / (luminance + 0.05),
    black = (luminance + 0.05) / 0.05;
  return {
    background: color,
    foreground: white >= black ? '#ffffff' : '#000000',
    contrast: Math.max(white, black),
  };
}
