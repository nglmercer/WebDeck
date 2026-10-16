import catalog from '../../../../contracts/keys.json';
export const keys = catalog.keys;
export function normalizeKey(value: string): string | undefined {
  const entry = keys.find(
    (k) =>
      k.id.toLowerCase() === value.toLowerCase() ||
      k.aliases.some((a) => a.toLowerCase() === value.toLowerCase()),
  );
  return (
    entry?.id ??
    ([...value].length === 1 && !/[\u0000-\u001f\u007f]/u.test(value) ? value : undefined)
  );
}
export function validateChord(values: string[]): string | undefined {
  if (!values.length || values.length > 16) return 'Choose between 1 and 16 keys.';
  const normalized = values.map(normalizeKey);
  if (normalized.some((k) => !k)) return 'Unknown key. Choose a key from the picker.';
  if (new Set(normalized).size !== normalized.length)
    return 'Remove duplicate keys or modifier aliases.';
  if (
    normalized.some(
      (id) => keys.find((k) => k.id === id)?.platformSupport.windows === 'unsupported',
    )
  )
    return 'This key is unavailable. Fn is usually handled by keyboard firmware. Use a dedicated media action when available.';
}
export function captureChord(event: KeyboardEvent): string[] | undefined {
  if (
    ['Control', 'Shift', 'Alt', 'Meta', 'AltGraph'].includes(event.key) ||
    event.isComposing ||
    event.repeat
  )
    return;
  // Function/navigation keys have stable codes. Characters follow the active layout;
  // KeyboardEvent.code is a physical position, not the character injected by Enigo.
  const candidate =
    /^(F\d+|Arrow\w+|Enter|Escape|Tab|Backspace|Delete|Insert|Home|End|PageUp|PageDown|Space)$/.test(
      event.code,
    )
      ? event.code === 'Space'
        ? 'space'
        : event.code
      : event.key;
  const key = normalizeKey(/^Numpad[0-9]$/.test(event.code) ? event.code.toLowerCase() : candidate);
  if (event.code.startsWith('Numpad') && !/^Numpad[0-9]$/.test(event.code)) return;
  if (!key) return;
  const chord = [
    ...(event.ctrlKey ? ['ctrl'] : []),
    ...(event.shiftKey ? ['shift'] : []),
    ...(event.altKey ? ['alt'] : []),
    ...(event.metaKey ? ['meta'] : []),
    key,
  ];
  return validateChord(chord) ? undefined : chord;
}
export const keyLabel = (id: string) => keys.find((k) => k.id === normalizeKey(id))?.label ?? id;

export function platformSupport(id: string, platform: string): string {
  const entry = keys.find((key) => key.id === normalizeKey(id));
  return entry
    ? ((entry.platformSupport as Record<string, string>)[platform] ?? 'unsupported')
    : 'unsupported';
}
