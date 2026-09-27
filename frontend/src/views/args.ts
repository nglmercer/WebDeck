import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import {
  asObject,
  asString,
  get,
  type BootContext,
  type JsonObject,
} from '../framework/types';
import { keyField } from '../components/keyfield';
import { parseField, type ArgSchema } from './argschema';

export { evalList } from './argschema';

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
  return `${cat}_${cmd}`;
}

/**
 * Render one parsed field. `fieldId` is the 1-based arg number and only
 * feeds dropdown option labels; every other kind ignores it.
 */
export function renderField(
  rctx: ArgsRenderContext,
  schema: ArgSchema,
  fieldId: string | number
): Html {
  const { ctx, argModalId } = rctx;
  const dark = ctx.dark_theme;
  const file = ''; // `{{file}}` is undefined upstream → renders ""

  switch (schema.kind) {
    case 'none':
      return raw('');
    case 'foldername': {
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
    case 'audioUpload':
      return html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept=".mp3" />`;
    case 'filetype':
      return join(
        schema.accepts.map(
          (filetypes) =>
            html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept="${filetypes.join(', ')}" />`
        )
      );
    case 'filepath':
      return join(
        schema.acceptLists.map(
          (filetypes) => html`<div class="filepath">
                <button class="filepath" filetypes="${filetypes.join('_')}"> ${text('select_your_file')} </button>
                <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
            </div>`
        )
      );
    case 'filePicker':
      return html`<div class="filepath">
        <button class="filepath"> ${text('select_your_file')} </button>
        <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
    </div>`;
    case 'folderPicker':
      return html`<div class="folderpath">
        <button class="folderpath"> ${text('select_your_file')} </button>
        <input type="text" class="folderpath ${raw(dark)}" placeholder="${text('no_file_chosen')}" />
    </div>`;
    case 'url':
      return html`<input class="${raw(dark)}" type="url" name="${file}" id="url_${argModalId}" placeholder="https://example.com" />`;
    case 'key':
      return keyField({ dark, id: argModalId, value: schema.value });
    case 'number':
      return join(
        schema.ranges.map(({ min, max }) => {
          if (min.startsWith('-') || max.startsWith('-')) {
            return html`<input class="${raw(dark)}" type="number" name="${file}" min="${min}" max="${max}" placeholder="${schema.placeholder}" />`;
          }
          return html`<input class="${raw(dark)}" type="number" pattern="[0-9]*" oninput="this.value = this.value.replace(/[^0-9]/g, '');" name="${file}" min="${min}" max="${max}" placeholder="${schema.placeholder}" />`;
        })
      );
    case 'longtext':
      return html`<textarea class="${raw(dark)}" name="${file}" rows="5" cols="33"></textarea>`;
    case 'usageTitle':
      return html`<input id="usage-title-input_${argModalId}" class="${raw(dark)}" type="text" name="${file}" size="10"
        ${schema.value !== '' ? html`value="${schema.value}"` : raw('')}
    />`;
    case 'text':
      return html`<input class="${raw(dark)}" type="text" name="${file}" size="10"
        ${schema.value !== '' ? html`value="${schema.value}"` : raw('')}
    />`;
    case 'hidden':
      return html`<input class="invisible" type="text" size="10" value="${schema.value}" />`;
    case 'dropdown': {
      const base = queryBase(rctx);
      const effectiveFieldId = String(fieldId) === '' ? 1 : fieldId;
      const options = schema.options.map((option, index) => {
        const optionId = index + 1;
        const optionName =
          option.label ?? text(`${base}__arg_${effectiveFieldId}_option_${optionId}_name`);
        return html`<option value="${option.id}"> ${optionName} </option>`;
      });
      return html`<select name="${file}">
        ${join(options)}
    </select>`;
    }
    case 'gpus': {
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
    case 'diskLetter': {
      const disks = asObject(get(ctx.usage_example, 'disks'));
      const options = Object.entries(disks).map(
        ([disk]) =>
          html`<option value="${disk}"${disk === 'C' ? raw('\n                    selected') : raw('')}> ${disk} </option>`
      );
      return html`<select id="disk-letter_${argModalId}" name="${file}">
        ${join(options)}
    </select>`;
    }
  }
}

/** Single arg field (one args.jinja include): parse, then render. */
export function argField(rctx: ArgsRenderContext, arg: JsonObject, argId: string | number): Html {
  return renderField(rctx, parseField(arg), argId);
}
