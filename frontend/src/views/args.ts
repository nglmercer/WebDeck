import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import {
  asArray,
  asObject,
  asString,
  get,
  type BootContext,
  type JsonObject,
} from '../framework/types';

/**
 * Port of the `eval(...)` literal parser used by args.jinja
 * (`['0','100']` → string list).
 */
export function evalList(source: string): string[] {
  const quoted = [...source.matchAll(/'([^']*)'|"([^"]*)"/g)].map((m) => m[1] ?? m[2] ?? '');
  if (quoted.length > 0) return quoted;
  return source
    .replace('[', '')
    .replace(']', '')
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s !== '');
}

export interface ArgsRenderContext {
  ctx: BootContext;
  category: string;
  command: string;
  subId: number;
  parentCommand: string;
  argModalId: string;
  commandValue: JsonObject;
}

function queryBase(rctx: ArgsRenderContext): string {
  const cat = rctx.category.replace(/ /g, '').toUpperCase();
  let cmd = rctx.command.replace(/ /g, '_').replace(/'/g, '').toLowerCase();
  if (rctx.subId !== 0) {
    cmd = rctx.parentCommand.replace(/ /g, '_').replace(/'/g, '').toLowerCase() + '_sub' + rctx.subId;
  }
  return `${cat}_${cmd}_`;
}

/** Single arg field (one args.jinja include). `argId` is the loop counter. */
export function argField(rctx: ArgsRenderContext, arg: JsonObject, argId: string | number): Html {
  const { ctx, argModalId } = rctx;
  const dark = ctx.dark_theme;
  const type = asString(arg['TYPE']);
  const file = ''; // `{{file}}` is undefined upstream → renders ""

  if (type.includes('webdeck_foldername')) {
    const folders = Object.keys(asObject(get(ctx.config, 'front', 'buttons')));
    return html`<div class="webdeck_foldername_ALL">
        ${join(
          folders.map(
            (key) => html`<div class="webdeck_foldername">
                <input class="${raw(dark)}" type="radio" name="file" value="${key}" />
                <label for="${key}">${key}</label>
            </div>`
          )
        )}
    </div>`;
  }
  if (type.includes('path-soundboard-audio')) {
    return html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept=".mp3" />`;
  }
  if (type.includes('filetype')) {
    const outputs: Html[] = [];
    for (const item of type.split(' ')) {
      if (item.startsWith('filetype')) {
        const filetypes = evalList(item.replace('filetype', ''));
        outputs.push(
          html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept="${filetypes.join(', ')}" />`
        );
      }
    }
    return join(outputs);
  }
  if (type.includes('filepath')) {
    const outputs: Html[] = [];
    for (const item of type.split(' ')) {
      if (item.startsWith('filepath')) {
        const filetypes = evalList(item.replace('filepath', ''));
        outputs.push(html`<div class="filepath">
                <button class="filepath" filetypes="${filetypes.join('_')}"> ${text('select_your_file')} </button>
                <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
            </div>`);
      }
    }
    return join(outputs);
  }
  if (type.includes('file')) {
    return html`<div class="filepath">
        <button class="filepath"> ${text('select_your_file')} </button>
        <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
    </div>`;
  }
  if (type.includes('folderpath')) {
    return html`<div class="folderpath">
        <button class="folderpath"> ${text('select_your_file')} </button>
        <input type="text" class="folderpath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
    </div>`;
  }
  if (type.includes('url')) {
    return html`<input class="${raw(dark)}" type="url" name="${file}" id="url_${argModalId}" placeholder="https://example.com" />`;
  }
  if (type.includes('number')) {
    const outputs: Html[] = [];
    for (const item of type.split(' ')) {
      if (item.startsWith('number')) {
        const placeholder = asString(arg['placeholder']);
        const numberList = evalList(item.replace('number', ''));
        const min = numberList[0] ?? '';
        const max = numberList[1] ?? '';
        if (min.startsWith('-') || max.startsWith('-')) {
          outputs.push(
            html`<input class="${raw(dark)}" type="number" name="${file}" min="${min}" max="${max}" placeholder="${placeholder}" />`
          );
        } else {
          outputs.push(
            html`<input class="${raw(dark)}" type="number" pattern="[0-9]*" oninput="this.value = this.value.replace(/[^0-9]/g, '');" name="${file}" min="${min}" max="${max}" placeholder="${placeholder}" />`
          );
        }
      }
    }
    return join(outputs);
  }
  if (type.includes('longtext') || type.includes('textarea')) {
    return html`<textarea class="${raw(dark)}" name="${file}" rows="5" cols="33"></textarea>`;
  }
  if (type.includes('usage-title-text') && type.includes('input')) {
    const value = asString(arg['value']);
    return html`<input id="usage-title-input_${argModalId}" class="${raw(dark)}" type="text" name="${file}" size="10"
        ${value !== '' ? html`value="${value}"` : raw('')}
    />`;
  }
  if (type.includes('text') && type.includes('input')) {
    const value = asString(arg['value']);
    return html`<input class="${raw(dark)}" type="text" name="${file}" size="10"
        ${value !== '' ? html`value="${value}"` : raw('')}
    />`;
  }
  if (type.includes('text') && asString(arg['value']) !== '') {
    return html`<input class="invisible" type="text" size="10" value="${asString(arg['value'])}" />`;
  }
  if (type.includes('dropdown')) {
    const base = queryBase(rctx);
    const effectiveArgId = String(argId) === '' ? 1 : argId;
    const options = asArray(arg['options']).map((option, index) => {
      const optionId = index + 1;
      const optionName = text(`${base}arg_${effectiveArgId}_option_${optionId}_name`);
      return html`<option value="${asString(asObject(option)['ID'])}"> ${optionName} </option>`;
    });
    return html`<select name="${file}">
        ${join(options)}
    </select>`;
  }
  if (type.includes('available_gpus')) {
    const gpus = asObject(get(ctx.usage_example, 'gpus'));
    const options = Object.entries(gpus).map(([gpu, usage]) => {
      const name = asString(asObject(usage)['name']);
      const label = name !== '' ? name : gpu;
      return html`<option value="${label}"> ${label} </option>`;
    });
    return html`<select name="${file}">
        ${join(options)}
    </select>`;
  }
  if (type.includes('disk-letter')) {
    const disks = asObject(get(ctx.usage_example, 'disks'));
    const options = Object.entries(disks).map(
      ([disk]) =>
        html`<option value="${disk}"${disk === 'C' ? raw('\n                    selected') : raw('')}> ${disk} </option>`
    );
    return html`<select id="disk-letter_${argModalId}" name="${file}">
        ${join(options)}
    </select>`;
  }
  return raw('');
}
