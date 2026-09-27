// Command browser + modal chrome (extracted from addbutton.ts).

import { html, join, raw, type Html } from '../../framework/html';
import { text } from '../../framework/i18n';
import {
  asArray,
  asObject,
  asString,
  rep,
  type BootContext,
} from '../../framework/types';
import { catKey, cmdKey } from '../args';
import { modalCloseIcon } from '../../components/icons';
import { addArgsModal } from './argsmodal';
import type { AddModalContext } from './types';

/** Command browser (index.jinja `.all-commands` block). */
export function addBrowserView(ctx: BootContext): Html {
  const commands = asObject(ctx.commands);
  const dark = ctx.dark_theme;

  const categories = Object.entries(commands).map(([category, categoryValue], catIndex) => {
    const catObj = asObject(categoryValue);
    const catQuery = `${catKey(category)}_CATEGORY_NAME`;
    let categoryName = text(catQuery);
    if (categoryName === catQuery || categoryName === '') categoryName = category;

    const items = Object.entries(catObj)
      .map(([command, commandValue], cmdIndex) => {
        if (command === 'CATEGORY-SETTINGS') return raw('');
        const cmdObj = asObject(commandValue);
        const argModalId = `${catIndex}X${cmdIndex}`;
        const type = asString(cmdObj['TYPE']);

        const descQuery = `${catKey(category)}_${cmdKey(command)}__${type.includes('multiple') ? 'category' : 'btn'}_description`;
        let buttonDescription = text(descQuery);
        // Plugin entries carry their doc text inline (no `.lang` entry).
        if (buttonDescription === descQuery) buttonDescription = asString(cmdObj['description']);
        const descBlock =
          buttonDescription !== ''
            ? html`<div class="addbutton-description"><p>${buttonDescription}</p></div>`
            : raw('');

        if (type !== 'multiple') {
          const titleQuery = `${catKey(category)}_${cmdKey(command)}__btn_name`;
          let buttonTitle = text(titleQuery);
          if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = command;
          const mctx: AddModalContext = {
            argModalId,
            category,
            command,
            parentCommand: '',
            subId: 0,
            commandValue: cmdObj,
            commandId: asString(cmdObj['command']),
            buttonTitle,
          };
          return join([
            descBlock,
            html`<button arg_modal_ID="${argModalId}" class="dropdown-btn no-dropdown ${raw(dark)}" id="open-button-${argModalId}" dropdown-commandTag="${command}">
              ${buttonTitle}
            </button>`,
            addArgsModal(ctx, mctx),
          ]);
        }

        // TYPE == multiple: sub-command dropdown.
        const titleQuery = `${catKey(category)}_${cmdKey(command)}__category_name`;
        let buttonTitle = text(titleQuery);
        if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = category;
        const subs = asArray(cmdObj['commands']).map((subValue, subIndex) => {
          const subObj = asObject(subValue);
          const subArgId = `${argModalId}X${subIndex}`;
          const subId = subIndex + 1;
          const subCommand = rep(asString(subObj['command']), '/', '');
          const subDescQuery = `${catKey(category)}_${cmdKey(command)}__category_description`;
          let subDesc = text(subDescQuery);
          if (subDesc === subDescQuery) subDesc = '';
          const subBtn = asString(subObj['TYPE']).includes('multiple') ? 'category' : 'btn';
          const subTitleQuery = `${catKey(category)}_${cmdKey(command)}_sub${subId}__${subBtn}_name`;
          // NOTE: upstream shadows `command` before building `_command`,
          // which is equivalent to parent + _sub{N} here.
          let subTitle = text(subTitleQuery);
          if (subTitle === subTitleQuery || subTitle === '') subTitle = subCommand;
          const mctx: AddModalContext = {
            argModalId: subArgId,
            category,
            command: subCommand,
            parentCommand: command,
            subId,
            commandValue: subObj,
            commandId: asString(subObj['command']),
            buttonTitle: subTitle,
          };
          return join([
            subDesc !== '' ? html`<div class="addbutton-description"><p>${subDesc}</p></div>` : raw(''),
            html`<button arg_modal_ID="${subArgId}" class="dropdown-btn no-dropdown ${raw(dark)}" id="open-button-${subArgId}" dropdown-commandTag="${subCommand}">
              ${subTitle}
            </button>`,
            addArgsModal(ctx, mctx),
          ]);
        });
        return join([
          descBlock,
          html`<button class="dropdown-btn ${raw(dark)}" dropdown-category="${command}">
            ${buttonTitle}
          </button>
          <div class="dropdown-container">
            <div class="dropdown-item-container">
              ${join(subs)}
            </div>
          </div>`,
        ]);
      });

    return join([
      html`<button class="dropdown-btn ${raw(dark)}" dropdown-category="${categoryName}">
        ${categoryName}
      </button>
      <div class="dropdown-container">
        ${join(items)}
      </div>`,
    ]);
  });

  return join(categories);
}

/** Add-button modal chrome (index.jinja addbutton section). */
export function addModalChrome(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  return html`
    <div class="addbutton-modal-container ${raw(dark)}">
      <div class="addbutton-modal-content ${raw(dark)}" id="addbutton-modal-content">
        <div class="addbutton-modal-header bold">
          <h1 class="addbutton-modal"> ${text('add_a_button')} </h1>
          <div class="addbutton-modal-close">
            ${modalCloseIcon('addbutton-config-modal', raw(dark))}
          </div>
        </div>
        <div class="addbutton-modal-main">
          <br class="addbutton-container ${raw(dark)}" />
          <div class="all-commands ${raw(dark)}">
            ${addBrowserView(ctx)}
          </div>
        </div>
      </div>
    </div>`;
}
