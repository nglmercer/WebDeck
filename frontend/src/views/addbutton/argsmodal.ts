// One args modal (extracted from addbutton.ts).
//
// Arg-field rendering itself lives in the shared ../args template; this
// module owns the modal shell, style block, and preview wiring.

import { html, raw, type Html } from '../../framework/html';
import { text } from '../../framework/i18n';
import {
  asArray,
  asObject,
  asString,
  type BootContext,
} from '../../framework/types';
import { editorSaveButton, editorStyleBlock } from '../../components/editor';
import { modalCloseIcon } from '../../components/icons';
import { catKey, cmdKey, renderArgsBlock } from '../args';
import { addPreview } from './preview';
import type { AddModalContext } from './types';

/** One args modal (addbutton_modal.jinja). */
export function addArgsModal(ctx: BootContext, mctx: AddModalContext): Html {
  const dark = ctx.dark_theme;
  const { argModalId: id, commandValue } = mctx;
  const args = asArray(commandValue['args']);
  const argsBlock = renderArgsBlock({
    ctx,
    category: mctx.category,
    command: mctx.command,
    subId: mctx.subId,
    parentCommand: mctx.parentCommand,
    commandValue,
    modalId: id,
    idAttr: 'arg_modal_ID',
  });

  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const btn: 'btn' | 'category' = asString(commandValue['TYPE']).includes('multiple') ? 'category' : 'btn';
  const buttonName = addButtonName(ctx, mctx, btn, hasStyle);

  const styleImage = asString(style['image']);
  const defaultSize =
    hasStyle && asString(style['image_size']).trim() !== ''
      ? asString(style['image_size']).trim().replace('%', '')
      : '75';

  return html`
<div class="addbutton-modal-container-args ${raw(dark)}" id="modal-container-${id}" arg_modal_ID="${id}">
  <div class="addbutton-modal-content-args ${raw(dark)}">
    <div class="addbutton-modal-header-args bold modal-container-${id}">
      <h1 class="addbutton-modal-args"> ${text('configure_your_button')}: ${mctx.buttonTitle}</h1>
      <div class="addbutton-modal-close-args">
        ${modalCloseIcon('addbutton-args-config-modal', raw(dark))}
      </div>
    </div>
    <div class="addbutton-modal-main-args">
      <div class="config-container ${raw(dark)}">
        <form class="args-form" arg_modal_ID="${id}" novalidate>
          ${argsBlock}
          ${args.length > 0 ? html`<div class="editorStyle-bar ${raw(dark)}"></div>` : raw('')}
          ${editorStyleBlock({
            dark,
            id,
            preview: addPreview(ctx, mctx, buttonName),
            defaultSize,
            backgroundColor: '',
            buttonName,
            nameValue: '',
          })}
          <div class="editorStyle-bar ${raw(dark)}" style="display: none;"></div>
          <div class="arg_container" arg_modal_ID="${id}" style="display: none;">
            <label for="command_${id}">Command (experimental):</label>
            <input class="${raw(dark)}" type="text" name="" id="command_${id}" value="${asString(commandValue['command'])}" readonly />
          </div>
          ${editorSaveButton(dark, id)}
        </form>
      </div>
    </div>
  </div>
</div>`;
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
