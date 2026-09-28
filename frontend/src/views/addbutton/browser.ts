// Command browser data (extracted from addbutton.ts). Markup in
// `AddBrowser.svelte` (tree) + `AddModal.svelte` (modal chrome).

import { text } from '../../framework/i18n';
import {
  asArray,
  asObject,
  asString,
  rep,
  type BootContext,
} from '../../framework/types';
import { catKey, cmdKey } from '../args';
import type { AddModalContext } from './types';

/** One leaf row: optional description + opener button + its args modal. */
export interface BrowserLeaf {
  desc: string;
  title: string;
  argModalId: string;
  commandTag: string;
  mctx: AddModalContext;
}

export interface BrowserBranch {
  desc: string;
  title: string;
  command: string;
  subs: BrowserLeaf[];
}

export type BrowserItem =
  | { kind: 'single'; leaf: BrowserLeaf }
  | { kind: 'multi'; branch: BrowserBranch };

export interface BrowserCategory {
  name: string;
  items: BrowserItem[];
}

export interface AddBrowserData {
  dark: string;
  categories: BrowserCategory[];
}

export function addBrowserData(ctx: BootContext): AddBrowserData {
  const commands = asObject(ctx.commands);
  const dark = ctx.dark_theme;

  const categories = Object.entries(commands).map(([category, categoryValue], catIndex) => {
    const catObj = asObject(categoryValue);
    const catQuery = `${catKey(category)}_CATEGORY_NAME`;
    let categoryName = text(catQuery);
    if (categoryName === catQuery || categoryName === '') categoryName = category;

    const items: BrowserItem[] = [];
    for (const [command, commandValue] of Object.entries(catObj)) {
      const cmdIndex = Object.keys(catObj).indexOf(command);
      if (command === 'CATEGORY-SETTINGS') continue;
      const cmdObj = asObject(commandValue);
      const argModalId = `${catIndex}X${cmdIndex}`;
      const type = asString(cmdObj['TYPE']);

      const descQuery = `${catKey(category)}_${cmdKey(command)}__${type.includes('multiple') ? 'category' : 'btn'}_description`;
      let buttonDescription = text(descQuery);
      // Plugin entries carry their doc text inline (no `.lang` entry).
      if (buttonDescription === descQuery) buttonDescription = asString(cmdObj['description']);

      if (type !== 'multiple') {
        const titleQuery = `${catKey(category)}_${cmdKey(command)}__btn_name`;
        let buttonTitle = text(titleQuery);
        if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = command;
        items.push({
          kind: 'single',
          leaf: {
            desc: buttonDescription,
            title: buttonTitle,
            argModalId,
            commandTag: command,
            mctx: {
              argModalId,
              category,
              command,
              parentCommand: '',
              subId: 0,
              commandValue: cmdObj,
              commandId: asString(cmdObj['command']),
              buttonTitle,
            },
          },
        });
        continue;
      }

      // TYPE == multiple: sub-command dropdown.
      const titleQuery = `${catKey(category)}_${cmdKey(command)}__category_name`;
      let buttonTitle = text(titleQuery);
      if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = category;
      const subs: BrowserLeaf[] = asArray(cmdObj['commands']).map((subValue, subIndex) => {
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
        return {
          desc: subDesc,
          title: subTitle,
          argModalId: subArgId,
          commandTag: subCommand,
          mctx: {
            argModalId: subArgId,
            category,
            command: subCommand,
            parentCommand: command,
            subId,
            commandValue: subObj,
            commandId: asString(subObj['command']),
            buttonTitle: subTitle,
          },
        };
      });
      items.push({ kind: 'multi', branch: { desc: buttonDescription, title: buttonTitle, command, subs } });
    }

    return { name: categoryName, items };
  });

  return { dark, categories };
}
