import type { TranslationKey } from '../../lib/translations';
import catalog from '../../../../contracts/keys.json';
import type { KeyboardCapabilities } from '../../lib/contracts';

export const keys = catalog.keys;
export const modifiers = ['ctrl', 'shift', 'alt', 'meta'] as const;
const modifierSet = new Set<string>(modifiers);

export function normalizeKey(value: string): string | undefined {
  const entry = keys.find(
    (key) =>
      key.id.toLowerCase() === value.toLowerCase() ||
      key.aliases.some((alias) => alias.toLowerCase() === value.toLowerCase()),
  );
  return (
    entry?.id ??
    ([...value].length === 1 && !/[\u0000-\u001f\u007f]/u.test(value) ? value : undefined)
  );
}

export function isModifier(value: string): boolean {
  return modifierSet.has(normalizeKey(value) ?? '');
}

/** Mirrors Rust's catalog, alias, duplicate and 1–16 key validation. */
export function validateChord(values: string[]): TranslationKey | undefined {
  if (!values.length || values.length > 16) return 'keyboard.chooseBetween1And16Keys';
  const normalized = values.map(normalizeKey);
  if (normalized.some((key) => !key)) return 'keyboard.unknownKey';
  if (new Set(normalized).size !== normalized.length) return 'keyboard.duplicateKeys';
  if (
    normalized.some((id) => {
      const entry = keys.find((key) => key.id === id);
      return (
        entry && Object.values(entry.platformSupport).every((support) => support === 'unsupported')
      );
    })
  )
    return 'keyboard.unavailableKey';
}

/** The standard picker uses modifiers plus one primary key; advanced mode is simultaneous. */
export function validatePickerChord(
  values: string[],
  advanced: boolean,
): TranslationKey | undefined {
  const chordError = validateChord(values);
  if (chordError) return chordError;
  const normalized = values.map((key) => normalizeKey(key)!);
  const modifierCount = normalized.filter((key) => modifierSet.has(key)).length;
  if (modifierCount > 4) return 'keyboard.tooManyModifiers';
  const primaryCount = normalized.length - modifierCount;
  if (primaryCount === 0) return 'keyboard.selectPrimary';
  if (primaryCount > 1 && !advanced) return 'keyboard.enableAdvanced';
}

export function validatePickerChordForHost(
  values: string[],
  advanced: boolean,
  capabilities?: KeyboardCapabilities,
): TranslationKey | undefined {
  const chordError = validatePickerChord(values, advanced);
  if (chordError || !capabilities) return chordError;
  if (capabilities.platform === 'unsupported' || capabilities.availability === 'unsupported')
    return 'keyboard.hostUnsupported';
  if (values.some((key) => platformSupport(key, capabilities.platform) === 'unsupported'))
    return 'keyboard.keyUnavailable';
}

export function toggleModifier(values: string[], modifier: string): string[] {
  const normalizedModifier = normalizeKey(modifier);
  if (!normalizedModifier || !modifierSet.has(normalizedModifier)) return values.slice();
  if (values.some((key) => normalizeKey(key) === normalizedModifier))
    return values.filter((key) => normalizeKey(key) !== normalizedModifier);

  const currentModifiers = values.filter(isModifier).map((key) => normalizeKey(key)!);
  const primaryKeys = values.filter((key) => !isModifier(key));
  return [...currentModifiers, normalizedModifier, ...primaryKeys];
}

export function selectPrimaryKey(values: string[], key: string, advanced: boolean): string[] {
  const normalizedKey = normalizeKey(key);
  if (!normalizedKey || modifierSet.has(normalizedKey)) return values.slice();
  const selected = values.some((value) => normalizeKey(value) === normalizedKey);
  if (advanced) {
    return selected
      ? values.filter((value) => normalizeKey(value) !== normalizedKey)
      : [...values, normalizedKey];
  }

  const primaryKeys = values.filter((value) => !isModifier(value));
  const currentModifiers = values.filter(isModifier).map((value) => normalizeKey(value)!);
  if (primaryKeys.length === 1 && normalizeKey(primaryKeys[0]!) === normalizedKey)
    return currentModifiers;
  return [...currentModifiers, normalizedKey];
}

/** Unknown hosts remain unknown; catalog mappings never imply live permission or output. */
export function platformSupport(id: string, platform?: string): string {
  const entry = keys.find((key) => key.id === normalizeKey(id));
  if (!entry) return 'unknown';
  if (Object.values(entry.platformSupport).every((support) => support === 'unsupported'))
    return 'unsupported';
  if (!platform || platform === 'unsupported') return 'unknown';
  return (entry.platformSupport as Record<string, string>)[platform] ?? 'unknown';
}

export function hostStatusMessage(capabilities?: KeyboardCapabilities): TranslationKey {
  if (!capabilities) return 'keyboard.hostUnknown';
  if (capabilities.platform === 'unsupported' || capabilities.availability === 'unsupported')
    return 'keyboard.hostUnsupported';
  if (
    capabilities.platform === 'linux_wayland' &&
    capabilities.availability === 'portal_approval_required'
  )
    return 'keyboard.permissionRequired';
  return 'keyboard.readyToTry';
}

export function keyStatusMessage(id: string, capabilities?: KeyboardCapabilities): TranslationKey {
  const support = platformSupport(id, capabilities?.platform);
  if (support === 'unsupported') return 'keyboard.keyUnavailable';
  if (!capabilities) return 'keyboard.hostUnknown';
  if (capabilities.platform === 'unsupported' || capabilities.availability === 'unsupported')
    return 'keyboard.hostUnsupported';
  if (support === 'unknown') return 'keyboard.hostUnknown';
  if (
    support === 'supported_if_portal_approved' ||
    (capabilities.platform === 'linux_wayland' &&
      capabilities.availability === 'portal_approval_required')
  )
    return 'keyboard.permissionRequired';
  return 'keyboard.readyToTry';
}

export function keyUnavailable(id: string, capabilities?: KeyboardCapabilities): boolean {
  const support = platformSupport(id, capabilities?.platform);
  return (
    support === 'unsupported' ||
    capabilities?.platform === 'unsupported' ||
    capabilities?.availability === 'unsupported'
  );
}

export function captureChord(event: KeyboardEvent): string[] | undefined {
  if (
    ['Control', 'Shift', 'Alt', 'Meta', 'AltGraph'].includes(event.key) ||
    event.isComposing ||
    event.repeat ||
    event.getModifierState?.('AltGraph')
  )
    return;

  // Named and function keys use stable codes; printable keys use the active layout's key value.
  const candidate =
    /^(F\d+|Arrow\w+|Enter|Escape|Tab|Backspace|Delete|Insert|Home|End|PageUp|PageDown|Space)$/.test(
      event.code,
    )
      ? event.code === 'Space'
        ? 'space'
        : event.code
      : event.key;
  if (event.code.startsWith('Numpad') && !/^Numpad[0-9]$/.test(event.code)) return;
  const key = normalizeKey(/^Numpad[0-9]$/.test(event.code) ? event.code.toLowerCase() : candidate);
  if (!key) return;

  const chord = [
    ...(event.ctrlKey ? ['ctrl'] : []),
    ...(event.shiftKey ? ['shift'] : []),
    ...(event.altKey ? ['alt'] : []),
    ...(event.metaKey ? ['meta'] : []),
    key,
  ];
  return validatePickerChord(chord, false) ? undefined : chord;
}

export const keyLabel = (id: string) =>
  keys.find((key) => key.id === normalizeKey(id))?.label ?? id;

export function sameKeys(left: string[], right: string[]): boolean {
  return left.length === right.length && left.every((key, index) => key === right[index]);
}
