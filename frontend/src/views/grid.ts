import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import {
  asArray,
  asBool,
  asObject,
  asString,
  get,
  pySplit,
  rep,
  repCount,
  type BootContext,
  type JsonObject,
} from '../framework/types';
import { editButtonModal } from './editmodal';
import { svgInlineStyle, svgSlot } from './svg';
import { addPlusIcon, deleteXIcon, editPencilIcon } from '../components/icons';

/**
 * `{% set onclick %}` + folder-button overrides. Returns the raw attribute
 * string for `<button>` plus the swap-mode folder div (rendered BEFORE the
 * form upstream).
 */
function buttonAttrs(message: string): { attrs: Html; folderDiv: Html } {
  if (message.startsWith('/folder')) {
    // Upstream builds the attr via {{ }} (autoescaped); escape the target.
    const target = rep(rep(message, '/folder ', ''), '"', '');
    const handler = html`folder(\`${target}\`)`;
    return {
      attrs: html`onclick="${handler}" onclickhandler="${handler}"`,
      folderDiv: html`<div class="swapMode-open-folder" onclick="${handler}" onclickhandler="${handler}" style="display: none;">
                    ${text('open_folder')}
                  </div>`,
    };
  }
  return { attrs: raw('type=submit'), folderDiv: raw('') };
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
function usageBlock(message: string, fill: string): Html {
  if (!message.startsWith('/usage')) return raw('');
  if ((pySplit(message)[1] ?? '') === '') return raw('');
  const replaced = repCount(rep(message, '<|§|>', ''), "'", '"', 2);
  const parts = replaced.split('"');
  const name = (parts[1] ?? '').trim();
  const path = parts[2] ?? '';
  const cls = rep(rep(rep(rep(path, "'", '"'), '"]["', '.'), '"]', ''), '["', '.');
  return html`
    <div class="usage">
      <div class="usage-title ${cls}" style="${fill}">
        ${name}
      </div>
      <div class="usage-value ${cls}" style="${fill}">
        -
      </div>
    </div>
  `;
}

function imagePath(image: string): string {
  if (image.startsWith('http')) return image;
  if (image.includes(':')) return 'static/img/' + (image.split('\\').pop() ?? image);
  if (image.startsWith('**uploaded/')) {
    return '.config/user_uploads/' + rep(image, '**uploaded/', '');
  }
  return 'static/img/' + image;
}

function gridButton(
  ctx: BootContext,
  folderId: string,
  buttonId: number,
  editModalId: string,
  entry: JsonObject
): Html {
  const message = asString(entry['message']);
  const { attrs, folderDiv } = buttonAttrs(message);
  const cls = buttonClass(asObject(get(ctx.config, 'front', 'buttons')), folderId, buttonId);
  const fill = fillStyle(entry);
  const showNames = asBool(get(ctx.config, 'front', 'show_names'));
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const buttontextStyle =
    namesColor !== '' && namesColor.trim() !== '' ? `style="color:${namesColor};"` : '';
  const name = asString(entry['name']);

  const image = asString(entry['image']);
  // Press-key buttons show their (short) label centered inside the tile
  // instead of below it — but only over an image, so the white overlay
  // text stays readable, and only when names are shown at all.
  const showInsideName = showNames && message.startsWith('/key') && image !== '' && name !== '';
  let media: Html;
  if (image === '') {
    media = raw('');
  } else {
    const imagepath = imagePath(image);
    const configuredSize = asString(entry['image_size']);
    const imageSize = configuredSize !== '' ? configuredSize : '70%';
    const sizeNum = parseInt(rep(imageSize, '%', ''), 10);
    const px = 112 * (sizeNum / 100) + 3;
    if (imagepath.endsWith('.svg')) {
      // Upstream guards with isfile(); the hydrator skips failed fetches.
      media = svgSlot(imagepath, svgInlineStyle(px, fill));
    } else {
      // onerror removal mirrors the isfile guard for missing files.
      media = html`<img src="${imagepath}" draggable="false" alt="${imagepath}" onerror="this.remove()" style="
                          width: ${String(px)}px;
                          ${fill}"
                        />`;
    }
  }

  return join([
    folderDiv,
    html`
              <form class="form-${String(buttonId)} form" id="${editModalId}">
                <div class="container-editmode">
                  <div class="edit-button" style="display: none;" edit_modal_ID="${editModalId}">
                    ${editPencilIcon()}
                  </div>
                  <div class="delete-button" style="display: none;">
                    ${deleteXIcon()}
                  </div>
                </div>
                <input type="hidden" class="message ${raw(ctx.dark_theme)}" value="${rep(message, '"', '&quot;')}" />
                <div class="checkbox" style="display: none;"></div>
                <button ${attrs} id="button_${editModalId}" edit_modal_ID="${editModalId}" class="${cls}" role="button" style="overflow: hidden; max-height: 89.6px;">
                  ${media}
                  ${showInsideName ? html`<span class="buttontext-inside">${name}</span>` : raw('')}
                  <!--|||||||||||||||||||||||-->
                  ${usageBlock(message, fill)}
                  <!--|||||||||||||||||||||||-->
                </button>
                ${
                  showNames && !showInsideName
                    ? html`<p class="buttontext" ${raw(buttontextStyle)}>
                    ${name}
                  </p>`
                    : raw('')
                }
              </form>`,
  ]);
}

function voidCell(folderId: string, buttonId: number, editModalId: string): Html {
  return html`
              <div class="void form-${String(buttonId)}" id="${editModalId}">
                <div class="checkbox" style="display: none;"></div>
                <div class="add-button" add_FOLDER="${folderId}" add_ID="${String(buttonId)}" style="display: none;">
                  ${addPlusIcon()}
                </div>
              </div>
  `;
}

/** Button grid + per-button edit modals (index.jinja folder loop). */
export function gridView(ctx: BootContext): Html {
  const buttons = asObject(get(ctx.config, 'front', 'buttons'));

  const folders = Object.entries(buttons).map(([folderId, value], folderIndex) => {
    const cells = asArray(value).map((buttonConfig, buttonId) => {
      const editModalId = `e${folderIndex}X${buttonId}`;
      const entry = asObject(buttonConfig);
      const isVoid = 'VOID' in entry || Object.keys(entry).length === 0;
      if (isVoid) return voidCell(folderId, buttonId, editModalId);
      const message = asString(entry['message']);
      // NOTE: the per-button edit modal is included AFTER the form upstream.
      // command_value always resolves to button_settings (the match loop is
      // scope-dead in both template engines), and command_id renders "".
      return join([
        gridButton(ctx, folderId, buttonId, editModalId, entry),
        editButtonModal(ctx, folderId, buttonId, editModalId, entry, message),
      ]);
    });
    // NOTE: duplicate id="folder-X" on both divs is upstream behavior.
    return html`
      <div id="folder-${folderId}" class="buttons-center invisible">
        <div id="folder-${folderId}" class="all-buttons">
          ${join(cells)}
        </div>
      </div>
    `;
  });
  return join(folders);
}
