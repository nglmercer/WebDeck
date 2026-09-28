// Preset icon registry: default button art + contrast styles.
//
// Two registries, one idea: nothing about button icons is hardcoded at the
// call site. Command presets (`webdeck/commands.json`, plugins) may ship a
// `style.image`; when they don't, the add-button flow resolves a registered
// default through `resolvePresetIcon` instead of rendering a blank tile.
// Tinted art goes through `iconFillStyle` instead of inline ternaries, so
// custom contrast tokens stay extendible the same way.
//
// The browser rows reuse the same source through `rowIcon` (every leaf),
// `branchIcon` (sub-groups) and `categoryIcon` (top-level groups), so each
// of the 81 commands — and every group containing them — shows an icon.

import type { SectionIconName } from './icons';

/** Identity of one command preset (a subset of `AddModalContext`). */
export interface PresetIconKey {
  category: string;
  command: string;
  parentCommand: string;
  commandId: string;
}

/** Resolved button art (mirrors the preset `style` keys). */
export interface PresetIcon {
  image: string;
  image_size: string;
}

/**
 * Registry matcher: an object matches when every defined field equals the
 * preset (`{}` matches everything, so it works as a global default), a
 * function matches on its own terms.
 */
export type PresetIconMatch =
  | {
      category?: string;
      command?: string;
      parentCommand?: string;
      commandId?: string;
    }
  | ((key: PresetIconKey) => boolean);

interface RegistryEntry {
  match: PresetIconMatch;
  /** `null` pins "no icon" (text-only tiles such as usage readouts). */
  icon: PresetIcon | null;
}

// Built-in defaults for the presets that ship no art. `Display/*` resolve
// to null on purpose: usage tiles are text-only by design (title + live
// value overlay), and an icon would collide with that overlay. The rest
// reuse matching glyphs from `static/img/` (or the purpose-drawn ones
// beside them). Anything still unmatched keeps today's blank rendering —
// register one instead of special-casing at the call site.
const builtinIcons: RegistryEntry[] = [
  { match: { category: 'Display' }, icon: null },
  {
    match: { commandId: '/mediacontrol previous' },
    icon: { image: 'skip-start.svg', image_size: '75%' },
  },
  {
    match: { commandId: '/mediacontrol playpause' },
    icon: { image: 'playpause.png', image_size: '75%' },
  },
  {
    match: { commandId: '/mediacontrol next' },
    icon: { image: 'skip-end.svg', image_size: '75%' },
  },
  { match: { category: 'System', command: 'Open' }, icon: { image: 'folder.png', image_size: '70%' } },
  { match: { category: 'System', command: 'opendir' }, icon: { image: 'folder.png', image_size: '70%' } },
  { match: { category: 'System', command: 'Kill a task' }, icon: { image: 'cross.svg', image_size: '70%' } },
  {
    match: { category: 'System', command: 'Restart a task' },
    icon: { image: 'reload.png', image_size: '70%' },
  },
  {
    match: { category: 'System', command: 'Execute batch code' },
    icon: { image: 'execscript.svg', image_size: '70%' },
  },
  {
    match: { category: 'System', command: 'Open a website' },
    icon: { image: 'globe.svg', image_size: '70%' },
  },
  { match: { category: 'System', command: 'Shutdown PC' }, icon: { image: 'power.svg', image_size: '70%' } },
  {
    match: { category: 'System', command: 'Restart PC' },
    icon: { image: 'restart.svg', image_size: '70%' },
  },
  { match: { category: 'System', command: 'Sleep PC' }, icon: { image: 'moon.svg', image_size: '70%' } },
  {
    match: { category: 'System', command: 'Hibernate PC' },
    icon: { image: 'hibernate.svg', image_size: '70%' },
  },
  {
    match: { commandId: '/spotify follow_or_unfollow_artist' },
    icon: { image: 'heart.svg', image_size: '70%' },
  },
  {
    match: { commandId: '/spotify follow_artist' },
    icon: { image: 'heart.svg', image_size: '70%' },
  },
  {
    match: { commandId: '/spotify unfollow_artist' },
    icon: { image: 'heart.svg', image_size: '70%' },
  },
];

/** Custom entries, newest first so the last registration wins. */
const customIcons: RegistryEntry[] = [];

/**
 * Register a default icon for presets matching `match`. Custom entries
 * take precedence over the built-ins; `icon: null` pins "no icon".
 */
export function registerPresetIcon(match: PresetIconMatch, icon: PresetIcon | null): void {
  customIcons.unshift({ match, icon });
}

/** Drop custom entries (tests); the built-ins are always present. */
export function resetPresetIcons(): void {
  customIcons.length = 0;
}

function isMatch(match: PresetIconMatch, key: PresetIconKey): boolean {
  if (typeof match === 'function') return match(key);
  return (
    (match.category === undefined || match.category === key.category) &&
    (match.command === undefined || match.command === key.command) &&
    (match.parentCommand === undefined || match.parentCommand === key.parentCommand) &&
    (match.commandId === undefined || match.commandId === key.commandId)
  );
}

function ownImage(style: { image?: unknown }): string {
  return typeof style.image === 'string' ? style.image : '';
}

function ownSize(style: { image_size?: unknown }): string {
  return typeof style.image_size === 'string' ? style.image_size : '';
}

/**
 * Resolve the art for one preset: the provided `style.image` always wins
 * (missing size falls back to `70%`, previously a `NaN`-width tile);
 * otherwise the first matching registry entry wins — a preset's own
 * `image_size` still wins over the default's; `null` means no icon.
 */
export function resolvePresetIcon(
  key: PresetIconKey,
  style: { image?: unknown; image_size?: unknown }
): PresetIcon | null {
  const image = ownImage(style);
  if (image !== '') {
    const size = ownSize(style);
    return { image, image_size: size !== '' ? size : '70%' };
  }
  for (const entry of [...customIcons, ...builtinIcons]) {
    if (!isMatch(entry.match, key)) continue;
    if (entry.icon === null) return null;
    const size = ownSize(style);
    return { image: entry.icon.image, image_size: size !== '' ? size : entry.icon.image_size };
  }
  return null;
}

/**
 * Seed a new button's state with its resolved default icon so the saved
 * tile matches the add-modal preview. Provided art is never touched.
 */
export function seedPresetButtonState(
  key: PresetIconKey,
  style: { image?: unknown; image_size?: unknown },
  button: Record<string, unknown>
): void {
  if (typeof button['image'] === 'string' && button['image'] !== '') return;
  const icon = resolvePresetIcon(key, style);
  if (icon === null) return;
  button['image'] = icon.image;
  button['image_size'] = icon.image_size;
}

// --- browser rows -----------------------------------------------------------

/**
 * One browser row icon: preset art (rendered in a light well, like the
 * deck tile it previews) or an inline section glyph (theme-colored).
 */
export type RowIcon = { kind: 'art'; image: string; fill: string } | { kind: 'glyph'; name: SectionIconName };

/**
 * Device glyphs for Display rows (browser only — usage tiles stay
 * text-only, so these deliberately live outside the tile registry).
 */
const DISPLAY_DEVICES: Array<{ match: PresetIconMatch; image: string }> = [
  { match: { category: 'Display', command: 'CPU' }, image: 'cpu.svg' },
  { match: { category: 'Display', parentCommand: 'Memory' }, image: 'memory.svg' },
  { match: { category: 'Display', parentCommand: 'Disks' }, image: 'disk.svg' },
  { match: { category: 'Display', parentCommand: 'GPU' }, image: 'gpu.svg' },
  { match: { category: 'Display', parentCommand: 'Network' }, image: 'network.svg' },
];

function ownColor(style: { color?: unknown }): string | undefined {
  return typeof style.color === 'string' ? style.color : undefined;
}

/**
 * Icon for one command (leaf) row: the resolved preset art, else the
 * Display device glyph, else the generic plus marker — every leaf gets
 * an icon, including art-less plugin presets.
 */
export function rowIcon(
  key: PresetIconKey,
  style: { image?: unknown; image_size?: unknown; color?: unknown }
): RowIcon {
  const fill = iconFillStyle(ownColor(style));
  const icon = resolvePresetIcon(key, style);
  if (icon !== null) return { kind: 'art', image: icon.image, fill };
  for (const device of DISPLAY_DEVICES) {
    if (isMatch(device.match, key)) return { kind: 'art', image: device.image, fill };
  }
  return { kind: 'glyph', name: 'plus' };
}

/** Built-in category glyphs (unknown/plugin categories fall back to `grid`). */
const CATEGORY_ICONS: Record<string, SectionIconName> = {
  Webdeck: 'grid',
  Display: 'chart',
  System: 'sliders',
  Text: 'text',
  Utilities: 'swatch',
  Soundboard: 'speaker',
  'OBS Studio': 'video',
  Spotify: 'music',
};

const customCategoryIcons = new Map<string, SectionIconName>();

/** Register (or override) the glyph for one browser category. */
export function registerCategoryIcon(category: string, name: SectionIconName): void {
  customCategoryIcons.set(category, name);
}

/** Drop custom category glyphs (tests). */
export function resetCategoryIcons(): void {
  customCategoryIcons.clear();
}

export function categoryIcon(category: string): SectionIconName {
  return customCategoryIcons.get(category) ?? CATEGORY_ICONS[category] ?? 'grid';
}

/** Identity of one sub-group (TYPE `multiple`) preset row. */
export interface BranchIconKey {
  category: string;
  branch: string;
}

export type BranchIconMatch =
  | { category?: string; branch?: string }
  | ((key: BranchIconKey) => boolean);

interface BranchEntry {
  match: BranchIconMatch;
  icon: RowIcon;
}

// Display branches share their device art with their leaves (group
// identity); volume/media branches use the matching inline glyph.
const builtinBranchIcons: BranchEntry[] = [
  {
    match: { category: 'Display', branch: 'Memory' },
    icon: { kind: 'art', image: 'memory.svg', fill: '' },
  },
  { match: { category: 'Display', branch: 'Disks' }, icon: { kind: 'art', image: 'disk.svg', fill: '' } },
  { match: { category: 'Display', branch: 'GPU' }, icon: { kind: 'art', image: 'gpu.svg', fill: '' } },
  {
    match: { category: 'Display', branch: 'Network' },
    icon: { kind: 'art', image: 'network.svg', fill: '' },
  },
  { match: { category: 'System', branch: 'Volume' }, icon: { kind: 'glyph', name: 'speaker' } },
  { match: { category: 'System', branch: 'Media control' }, icon: { kind: 'glyph', name: 'play' } },
  {
    match: { category: 'Spotify', branch: 'Volume (ONLY WITH SPOTIFY PREMIUM)' },
    icon: { kind: 'glyph', name: 'speaker' },
  },
];

/** Custom branch entries, newest first so the last registration wins. */
const customBranchIcons: BranchEntry[] = [];

/** Register (or override) the icon for sub-groups matching `match`. */
export function registerBranchIcon(match: BranchIconMatch, icon: RowIcon): void {
  customBranchIcons.unshift({ match, icon });
}

/** Drop custom branch icons (tests). */
export function resetBranchIcons(): void {
  customBranchIcons.length = 0;
}

function isBranchMatch(match: BranchIconMatch, key: BranchIconKey): boolean {
  if (typeof match === 'function') return match(key);
  return (
    (match.category === undefined || match.category === key.category) &&
    (match.branch === undefined || match.branch === key.branch)
  );
}

/** Icon for one sub-group row (unknown groups fall back to `grid`). */
export function branchIcon(key: BranchIconKey): RowIcon {
  for (const entry of [...customBranchIcons, ...builtinBranchIcons]) {
    if (isBranchMatch(entry.match, key)) return entry.icon;
  }
  return { kind: 'glyph', name: 'grid' };
}

// --- contrast ---------------------------------------------------------------

const DEFAULT_CONTRAST: Array<[string, string]> = [['invert', 'filter: invert(1)']];

/** Special color tokens → raw style (replaces the inline ternaries). */
const contrastStyles = new Map<string, string>(DEFAULT_CONTRAST);

/** Register (or override) a contrast token such as `'invert'`. */
export function registerContrastStyle(token: string, style: string): void {
  contrastStyles.set(token, style);
}

/** Drop custom tokens (tests); the defaults are always restored. */
export function resetContrastStyles(): void {
  contrastStyles.clear();
  for (const [token, style] of DEFAULT_CONTRAST) contrastStyles.set(token, style);
}

/**
 * Inline style for tinted button art: registered tokens map to their
 * style, any other color tints `fill` + `color`, empty/absent means none.
 */
export function iconFillStyle(color: string | undefined): string {
  if (color === undefined || color === '') return '';
  const registered = contrastStyles.get(color);
  if (registered !== undefined) return registered;
  return `fill:${color}; color:${color};`;
}
