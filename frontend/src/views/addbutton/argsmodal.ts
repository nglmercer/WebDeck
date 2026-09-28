// One args modal (extracted from addbutton.ts). Markup in `AddArgsModal.svelte`.

import { text } from '../../framework/i18n';
import { asObject, asString, type BootContext } from '../../framework/types';
import { resolvePresetIcon } from '../../components/button-icons';
import type { PreviewData } from '../../components/preview';
import { catKey, cmdKey, hasVisibleParams } from '../args';
import { addPreviewData } from './preview';
import type { AddModalContext } from './types';

export interface AddArgsData {
  dark: string;
  id: string;
  buttonTitle: string;
  hasArgs: boolean;
  defaultSize: string;
  buttonName: string;
  preview: PreviewData;
  command: string;
}

export function addArgsData(ctx: BootContext, mctx: AddModalContext): AddArgsData {
  const dark = ctx.dark_theme;
  const { argModalId: id, commandValue } = mctx;

  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const btn: 'btn' | 'category' = asString(commandValue['TYPE']).includes('multiple') ? 'category' : 'btn';
  const buttonName = addButtonName(ctx, mctx, btn, hasStyle);

  const providedSize = asString(style['image_size']).trim();
  const icon = resolvePresetIcon(mctx, style);
  const defaultSize =
    providedSize !== ''
      ? providedSize.replace('%', '')
      : icon !== null
        ? icon.image_size.replace('%', '')
        : '75';

  return {
    dark,
    id,
    buttonTitle: mctx.buttonTitle,
    hasArgs: hasVisibleParams(commandValue),
    defaultSize,
    buttonName,
    preview: addPreviewData(ctx, mctx, buttonName),
    command: asString(commandValue['command']),
  };
}

export function addButtonName(ctx: BootContext, mctx: AddModalContext, btn: 'btn' | 'category', hasStyle: boolean): string {
  void ctx;
  const base =
    mctx.subId !== 0
      ? `${catKey(mctx.category)}_${cmdKey(mctx.parentCommand)}_sub${mctx.subId}`
      : `${catKey(mctx.category)}_${cmdKey(mctx.command)}`;
  if (hasStyle) {
    const displayQuery = `${base}__${btn}_default_display_name`;
    const displayName = text(displayQuery);
    if (displayName !== displayQuery && displayName !== '') return displayName;
  }
  return text(`${base}__${btn}_name`);
}
