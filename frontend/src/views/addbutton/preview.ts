// Button preview tile (extracted from addbutton.ts).

import { html, join, raw, type Html } from '../../framework/html';
import {
  asObject,
  asString,
  get,
  rep,
  type BootContext,
} from '../../framework/types';
import { svgSlot } from '../svg';
import type { AddModalContext } from './types';

export function addPreview(ctx: BootContext, mctx: AddModalContext, buttonName: string): Html {
  const dark = ctx.dark_theme;
  const { argModalId: id, commandValue } = mctx;
  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const buttontextStyle =
    namesColor !== '' && namesColor.trim() !== '' ? `style="color:${namesColor};"` : '';

  if (!hasStyle) {
    return join([
      html`<button type="button" class="wd_button" role="button" style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;">
                      <img id="button-image_${id}" draggable="false" alt="" style="
                        width: ${String(112 * (50 / 100) + 3)}px;
                      " />
                      <div class="usage">
                        <div id="usage-title_${id}" class="usage-title" style=""></div>
                        <div id="usage-value_${id}" class="usage-value" style=""></div>
                      </div>
                    </button>`,
      html`<p class="buttontext" id="button-text-preview_${id}">
                      ${buttonName}
                    </p>`,
    ]);
  }

  let fill = '';
  if ('color' in style) {
    const color = asString(style['color']);
    fill = color === 'invert' ? 'filter: invert(1)' : `fill:${color}; color:${color};`;
  }
  const styleImage = asString(style['image']);
  let media: Html;
  if (styleImage === '') {
    media = html`<img id="button-image_${id}" draggable="false" alt="" style="width: ${String(112 * (50 / 100) + 3)}px;" />`;
  } else {
    const imagelink = addImageLink(styleImage);
    const sizeNum = parseInt(rep(asString(style['image_size']), '%', ''), 10);
    const px = 112 * (sizeNum / 100) + 3;
    media = imagelink.endsWith('.svg')
      ? svgSlot(imagelink, ` id="button-image_${id}" style="width:${px}px; height:${px}; ${fill}"`, '<svg')
      : html`<img id="button-image_${id}" src="${imagelink}" draggable="false" style="
                            width: ${String(px)}px;
                            ${fill}"
                          />`;
  }
  return join([
    html`<button type="button" id="button-element_${id}" class="wd_button" role="button" style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;">
                      ${media}
                      <div class="usage">
                        <div id="usage-title_${id}" class="usage-title" style="${fill}"></div>
                        <div id="usage-value_${id}" class="usage-value" style="${fill}"></div>
                      </div>
                    </button>`,
    html`<p class="buttontext" id="button-text-preview_${id}" ${raw(buttontextStyle)}>
                      ${buttonName}
                    </p>`,
  ]);
}

function addImageLink(styleImage: string): string {
  if (styleImage.startsWith('http')) return styleImage;
  if (styleImage.includes(':')) return 'static/img/' + (styleImage.split('\\').pop() ?? styleImage);
  if (styleImage.startsWith('**uploaded/')) {
    // Upstream reads an out-of-scope config path here (renders broken);
    // use the style image itself so uploaded art actually previews.
    return '.config/user_uploads/' + rep(styleImage, '**uploaded/', '');
  }
  return 'static/img/' + styleImage;
}
