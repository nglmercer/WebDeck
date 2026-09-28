import { asArray, asBool, asObject, asString, get, pySplit, rep, repCount } from '../framework/types';
import type { BootContext, JsonObject } from '../framework/types';
import { text } from '../framework/i18n';
import { svgInlineStyle, svgSlotId } from './svg';

/**
 * Grid data model (index.jinja folder loop). Pure data in, markup out in
 * `Grid.svelte`. Every transform below is the byte-identical logic the
 * string `gridView` used.
 */

export interface UsageData {
  name: string;
  cls: string;
  fill: string;
}

export type MediaData =
  | { kind: 'none' }
  | { kind: 'img'; src: string; px: number; fill: string }
  | { kind: 'svg'; slot: number };

export interface ButtonCellData {
  kind: 'button';
  folderId: string;
  buttonId: number;
  editModalId: string;
  cls: string;
  name: string;
  /** Raw `folder(...)` handler source for /folder buttons, else null. */
  folderHandler: string | null;
  /** Hidden input value (`"` pre-replaced, exactly as parsed upstream). */
  hiddenValue: string;
  dark: string;
  media: MediaData;
  showInsideName: boolean;
  usage: UsageData | null;
  showBelowName: boolean;
  /** `color:X;` for the below-tile name, else null. */
  namesColorStyle: string | null;
  /** Raw entry + message for the per-button edit modal. */
  entry: JsonObject;
  message: string;
}

export interface VoidCellData {
  kind: 'void';
  folderId: string;
  buttonId: number;
  editModalId: string;
}

export type CellData = ButtonCellData | VoidCellData;

export interface FolderData {
  folderId: string;
  cells: CellData[];
}

export interface GridData {
  folders: FolderData[];
  openFolder: string;
}

/** `{% set onclick %}` + folder-button overrides (raw handler source). */
function folderHandlerFor(message: string): string | null {
  if (!message.startsWith('/folder')) return null;
  // Upstream builds the attr via {{ }} (autoescaped); escape the target.
  const target = rep(rep(message, '/folder ', ''), '"', '');
  return `folder(\`${target}\`)`;
}

function buttonClass(buttons: JsonObject, folderId: string, buttonId: number): string {
  let cls = 'wd_button';
  const entry = asObject(asArray(buttons[folderId])[buttonId]);
  if ('background_color' in entry) {
    cls += ' button-' + rep(asString(entry['background_color']), '#', '');
  }
  const message = entry['message'];
  if (typeof message === 'string' && message !== '') {
    const stripped = message.trim();
    if (stripped === '/fullscreen') cls += ' fullscreen-btn';
    if (stripped === '/zoom in') cls += ' zoom-in-btn';
    if (stripped === '/zoom out') cls += ' zoom-out-btn';
    if (stripped === '/buttons-div expand') cls += ' expand-btn';
    if (stripped === '/buttons-div shrink') cls += ' shrink-btn';
    if (stripped === '/box-zoom-out') cls += ' box-zoom-out';
    if (stripped === '/box-zoom-in') cls += ' box-zoom-in';
    if (stripped === '/page-dezoom') cls += ' dezoom-btn';
    if (stripped === '/page-zoom') cls += ' zoom-btn';
    if (stripped === '/open-modal') cls += ' open-config-modal';
    if (stripped === '/open-config') cls += ' open-config-modal';
    if (stripped === '/open-config-modal') cls += ' open-config-modal';
  }
  return cls;
}

function fillStyle(entry: JsonObject): string {
  if (!('color' in entry)) return '';
  const color = asString(entry['color']);
  if (color === 'invert') return 'filter: invert(1)';
  return `fill:${color}; color:${color};`;
}

/** Usage title/value block shared by both image branches. */
function usageData(message: string, fill: string): UsageData | null {
  if (!message.startsWith('/usage')) return null;
  if ((pySplit(message)[1] ?? '') === '') return null;
  const replaced = repCount(rep(message, '<|§|>', ''), "'", '"', 2);
  const parts = replaced.split('"');
  const name = (parts[1] ?? '').trim();
  const path = parts[2] ?? '';
  const cls = rep(rep(rep(rep(path, "'", '"'), '"]["', '.'), '"]', ''), '["', '.');
  return { name, cls, fill };
}

function imagePath(image: string): string {
  if (image.startsWith('http')) return image;
  if (image.includes(':')) return 'static/img/' + (image.split('\\').pop() ?? image);
  if (image.startsWith('**uploaded/')) {
    return '.config/user_uploads/' + rep(image, '**uploaded/', '');
  }
  return 'static/img/' + image;
}

function mediaData(entry: JsonObject, fill: string): MediaData {
  const image = asString(entry['image']);
  if (image === '') return { kind: 'none' };
  const imagepath = imagePath(image);
  const configuredSize = asString(entry['image_size']);
  const imageSize = configuredSize !== '' ? configuredSize : '70%';
  const sizeNum = parseInt(rep(imageSize, '%', ''), 10);
  const px = 112 * (sizeNum / 100) + 3;
  if (imagepath.endsWith('.svg')) {
    // Upstream guards with isfile(); the hydrator skips failed fetches.
    return { kind: 'svg', slot: svgSlotId(imagepath, svgInlineStyle(px, fill)) };
  }
  return { kind: 'img', src: imagepath, px, fill };
}

function buttonCell(
  ctx: BootContext,
  buttons: JsonObject,
  folderId: string,
  buttonId: number,
  editModalId: string,
  entry: JsonObject
): ButtonCellData {
  const message = asString(entry['message']);
  const fill = fillStyle(entry);
  const showNames = asBool(get(ctx.config, 'front', 'show_names'));
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const name = asString(entry['name']);
  const image = asString(entry['image']);
  // Press-key buttons show their (short) label centered inside the tile
  // instead of below it — but only over an image, so the white overlay
  // text stays readable, and only when names are shown at all.
  const showInsideName = showNames && message.startsWith('/key') && image !== '' && name !== '';
  return {
    kind: 'button',
    folderId,
    buttonId,
    editModalId,
    cls: buttonClass(buttons, folderId, buttonId),
    name,
    folderHandler: folderHandlerFor(message),
    hiddenValue: rep(message, '"', '&quot;'),
    dark: ctx.dark_theme,
    media: mediaData(entry, fill),
    showInsideName,
    usage: usageData(message, fill),
    showBelowName: showNames && !showInsideName,
    namesColorStyle: namesColor !== '' && namesColor.trim() !== '' ? `color:${namesColor};` : null,
    // NOTE: the per-button edit modal renders AFTER the form upstream.
    entry,
    message,
  };
}

/** Button grid data + per-button edit modals (index.jinja folder loop). */
export function gridData(ctx: BootContext): GridData {
  const buttons = asObject(get(ctx.config, 'front', 'buttons'));
  const folders = Object.entries(buttons).map(([folderId, value], folderIndex) => {
    const cells = asArray(value).map((buttonConfig, buttonId): CellData => {
      const editModalId = `e${folderIndex}X${buttonId}`;
      const entry = asObject(buttonConfig);
      const isVoid = 'VOID' in entry || Object.keys(entry).length === 0;
      if (isVoid) return { kind: 'void', folderId, buttonId, editModalId };
      return buttonCell(ctx, buttons, folderId, buttonId, editModalId, entry);
    });
    return { folderId, cells };
  });
  return { folders, openFolder: text('open_folder') };
}
