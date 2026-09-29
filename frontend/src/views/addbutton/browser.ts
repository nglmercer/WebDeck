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
import {
  branchIcon,
  categoryIcon,
  rowIcon,
  type RowIcon,
} from '../../components/button-icons';
import type { SectionIconName } from '../../components/icons';
import { previewImageLink } from '../../components/preview';
import { catKey, cmdKey } from '../args';
import { svgInlineStyle, svgSlotId } from '../svg';
import type { AddModalContext } from './types';

/** Render-ready row icon: art resolves to an img or svg slot, glyphs inline. */
export type BrowserRowIcon =
  | { kind: 'img'; src: string; fill: string }
  | { kind: 'svg'; slot: number }
  | { kind: 'glyph'; name: SectionIconName };

/** Row art edge, in px (rendered inside the light icon well). */
const ROW_ICON_PX = 20;

function toRowIcon(icon: RowIcon): BrowserRowIcon {
  if (icon.kind === 'glyph' || icon.image === '') {
    return { kind: 'glyph', name: icon.kind === 'glyph' ? icon.name : 'plus' };
  }
  const src = previewImageLink(icon.image);
  if (src.endsWith('.svg')) {
    return { kind: 'svg', slot: svgSlotId(src, svgInlineStyle(ROW_ICON_PX, icon.fill)) };
  }
  return { kind: 'img', src, fill: icon.fill };
}

/** One leaf row: optional description + opener button + its args modal. */
export interface BrowserLeaf {
  desc: string;
  title: string;
  argModalId: string;
  commandTag: string;
  icon: BrowserRowIcon;
  mctx: AddModalContext;
}

export interface BrowserBranch {
  desc: string;
  title: string;
  command: string;
  icon: BrowserRowIcon;
  subs: BrowserLeaf[];
}

export type BrowserItem =
  | { kind: 'single'; leaf: BrowserLeaf }
  | { kind: 'multi'; branch: BrowserBranch };

export interface BrowserCategory {
  name: string;
  icon: SectionIconName;
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
    for (const [cmdIndex, [command, commandValue]] of Object.entries(catObj).entries()) {
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
        items.push({
          kind: 'single',
          leaf: {
            desc: buttonDescription,
            title: buttonTitle,
            argModalId,
            commandTag: command,
            icon: toRowIcon(rowIcon(mctx, asObject(cmdObj['style']))),
            mctx,
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
        return {
          desc: subDesc,
          title: subTitle,
          argModalId: subArgId,
          commandTag: subCommand,
          icon: toRowIcon(rowIcon(mctx, asObject(subObj['style']))),
          mctx,
        };
      });
      items.push({
        kind: 'multi',
        branch: {
          desc: buttonDescription,
          title: buttonTitle,
          command,
          icon: toRowIcon(branchIcon({ category, branch: command })),
          subs,
        },
      });
    }

    return { name: categoryName, icon: categoryIcon(category), items };
  });

  return { dark, categories };
}
