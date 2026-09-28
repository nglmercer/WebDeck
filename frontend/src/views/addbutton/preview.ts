// Button preview tile data (extracted from addbutton.ts).

import { iconFillStyle, resolvePresetIcon } from '../../components/button-icons';
import { previewImageLink, type PreviewData, type PreviewMedia } from '../../components/preview';
import {
  asObject,
  asString,
  get,
  rep,
  type BootContext,
} from '../../framework/types';
import { svgInlineStyle, svgSlotId } from '../svg';
import type { AddModalContext } from './types';

export function addPreviewData(ctx: BootContext, mctx: AddModalContext, buttonName: string): PreviewData {
  const { argModalId: id, commandValue } = mctx;
  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  // The style-less tile never carries the names color (upstream parity).
  const textStyle =
    hasStyle && namesColor !== '' && namesColor.trim() !== '' ? `color:${namesColor};` : null;

  const fill = 'color' in style ? iconFillStyle(asString(style['color'])) : '';
  // Provided art wins; otherwise the registry supplies the preset's
  // default icon (usage presets resolve null: text-only by design).
  const icon = resolvePresetIcon(mctx, style);
  let media: PreviewMedia;
  if (icon === null) {
    media = {
      kind: 'img',
      src: null,
      alt: '',
      removeOnError: false,
      widthPx: 112 * (50 / 100) + 3,
      fill: '',
    };
  } else {
    const imagelink = previewImageLink(icon.image);
    const sizeNum = parseInt(rep(icon.image_size, '%', ''), 10);
    const px = 112 * (sizeNum / 100) + 3;
    media = imagelink.endsWith('.svg')
      ? {
          kind: 'svg',
          slot: svgSlotId(imagelink, ` id="button-image_${id}" ${svgInlineStyle(px, fill)}`, '<svg'),
        }
      : { kind: 'img', src: imagelink, alt: null, removeOnError: false, widthPx: px, fill };
  }

  return {
    id,
    buttonId: hasStyle || icon !== null,
    buttonStyle: 'overflow: hidden; overflow-y: hidden; max-height: 89.6px;',
    media,
    usageFill: fill,
    text: buttonName,
    textStyle,
  };
}
