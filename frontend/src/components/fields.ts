/**
 * Shared field types + helpers. All field markup migrated to Svelte
 * components (`SwitchField.svelte`, ...); this module keeps the option
 * type and the color normalizer they share.
 */

/** Labeled dropdown option. */
export interface SelectOption {
  value: string;
  label: string;
  selected: boolean;
}

/** Normalize a configured color to `#rrggbb` (or '' when unset). */
export function normalizeHexColor(value: string): string {
  const trimmed = value.trim();
  if (trimmed === '') return '';
  return trimmed.startsWith('#') ? trimmed : `#${trimmed}`;
}
