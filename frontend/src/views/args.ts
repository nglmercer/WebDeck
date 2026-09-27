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
import { q } from '../query';
import { keyField } from '../components/keyfield';
import { consumesArgNumber, parseArg, parseField, type ArgSchema } from './argschema';

export { evalList } from './argschema';

/** Translation-key segments for category/command names (shared add/edit). */
export function catKey(category: string): string {
  return category.replace(/ /g, '').toUpperCase();
}

export function cmdKey(command: string): string {
  return command.replace(/ /g, '_').replace(/'/g, '').toLowerCase();
}

/** Modal-id attribute flavor: add modals use `arg_modal_ID`, edit modals `edit_modal_ID`. */
export type ModalIdAttr = 'arg_modal_ID' | 'edit_modal_ID';

export interface ArgsRenderContext {
  ctx: BootContext;
  category: string;
  command: string;
  subId: number;
  parentCommand: string;
  argModalId: string;
  idAttr: ModalIdAttr;
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
 * Saved values for one args form (edit modal), aligned to collection order:
 * `values[i]` feeds the i-th collected position. `choices` overrides the
 * selected option per top-level choice arg. Absent in add modals.
 */
export interface ArgsPrefill {
  values: string[];
  choices: Map<number, number>;
}

/** Render-order cursor over an {@link ArgsPrefill}. */
export interface PrefillCursor {
  prefill: ArgsPrefill;
  pos: number;
}

export function nextPrefill(cursor: PrefillCursor | undefined): string | undefined {
  if (!cursor || cursor.pos >= cursor.prefill.values.length) return undefined;
  return cursor.prefill.values[cursor.pos++];
}

/**
 * Collect an args form's values in DOM order (shared add/edit submit path).
 * Inputs stamped with `data-preserved` (values the form cannot represent,
 * e.g. a previously picked upload filename) contribute it when empty.
 */
export function collectArgValues(container: Element): string[] {
  return q(container)
    .find('input, select, textarea')
    .toArray()
    .filter((input) => {
      if (q(input).parent().css('display') === 'none') {
        return false;
      }
      if (q(input).hasClass('choice')) {
        return false;
      }
      if (q(input).hasClass('key-aux')) {
        return false;
      }
      if (q(input).closest('.editorStyle, .webdeck_foldername_div').length > 0) {
        return false;
      }
      return true;
    })
    .map((input) => {
      if (q(input).is('select')) {
        const select = input as HTMLSelectElement;
        return select.options[select.selectedIndex]?.value ?? '';
      }
      const field = input as HTMLInputElement;
      const kind = q(field).prop('type');
      if (kind === 'radio' || kind === 'checkbox') {
        return q(field).prop('checked') === true ? String(q(field).val() ?? '') : '';
      } else if (kind === 'submit' || kind === 'button') {
        return '';
      } else {
        const current = String(q(field).val() ?? '');
        if (current === '') {
          const preserved = q(field).attr('data-preserved');
          if (preserved) return preserved;
        }
        if (current.startsWith('/folder')) {
          const stripped = current.replace(/"/g, '');
          q(field).val(stripped);
          return stripped;
        }
        return current;
      }
    })
    .filter((value) => value !== '');
}

/** `commandId` plus collected values, joined for the backend. */
export function buildCommand(commandId: string, container: Element): string {
  return commandId + ' ' + collectArgValues(container).join('<|§|>');
}

/**
 * Render one parsed field. `fieldId` is the 1-based arg number and only
 * feeds dropdown option labels; every other kind ignores it. `cursor`
 * feeds saved values in collection order (edit modal; absent when adding).
 */
export function renderField(
  rctx: ArgsRenderContext,
  schema: ArgSchema,
  fieldId: string | number,
  cursor?: PrefillCursor
): Html {
  const { ctx, argModalId } = rctx;
  const dark = ctx.dark_theme;
  const file = ''; // `{{file}}` is undefined upstream → renders ""

  switch (schema.kind) {
    case 'none':
      return raw('');
    case 'foldername': {
      const folders = Object.keys(asObject(get(ctx.config, 'front', 'buttons')));
      const checked = nextPrefill(cursor);
      return html`<div class="webdeck_foldername_ALL">
        ${join(
          folders.map(
            (key) => html`<div class="webdeck_foldername">
                <input${checked !== undefined && key === checked ? raw(' checked') : raw('')} class="${raw(dark)}" type="radio" name="file" value="${key}" />
                <label for="${key}">${key}</label>
            </div>`
          )
        )}
    </div>`;
    }
    case 'audioUpload': {
      const preserved = nextPrefill(cursor);
      return html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept=".mp3"${preserved !== undefined ? html` data-preserved="${preserved}"` : raw('')} />`;
    }
    case 'filetype':
      return join(
        schema.accepts.map((filetypes) => {
          const preserved = nextPrefill(cursor);
          return html`<input class="${raw(dark)} audio-input" id="audio-input_${argModalId}" type="file" name="file" accept="${filetypes.join(', ')}"${preserved !== undefined ? html` data-preserved="${preserved}"` : raw('')} />`;
        })
      );
    case 'filepath':
      return join(
        schema.acceptLists.map((filetypes) => {
          const value = nextPrefill(cursor);
          return html`<div class="filepath">
                <button class="filepath" filetypes="${filetypes.join('_')}"> ${text('select_your_file')} </button>
                <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}"${value !== undefined ? html` value="${value}"` : raw('')} />
            </div>`;
        })
      );
    case 'filePicker': {
      const value = nextPrefill(cursor);
      return html`<div class="filepath">
        <button class="filepath"> ${text('select_your_file')} </button>
        <input type="text" class="filepath ${raw(dark)}" placeholder="${text('no_file_chosen')}"${value !== undefined ? html` value="${value}"` : raw('')} />
    </div>`;
    }
    case 'folderPicker': {
      const value = nextPrefill(cursor);
      return html`<div class="folderpath">
        <button class="folderpath"> ${text('select_your_file')} </button>
        <input type="text" class="folderpath ${raw(dark)}" placeholder="${text('no_file_chosen')}"${value !== undefined ? html` value="${value}"` : raw('')} />
    </div>`;
    }
    case 'url': {
      const value = nextPrefill(cursor);
      return html`<input class="${raw(dark)}" type="url" name="${file}" id="url_${argModalId}" placeholder="https://example.com"${value !== undefined ? html` value="${value}"` : raw('')} />`;
    }
    case 'key':
      return keyField({ dark, id: argModalId, value: nextPrefill(cursor) ?? schema.value });
    case 'number':
      return join(
        schema.ranges.map(({ min, max }) => {
          const value = nextPrefill(cursor);
          const preset = value !== undefined ? html` value="${value}"` : raw('');
          if (min.startsWith('-') || max.startsWith('-')) {
            return html`<input class="${raw(dark)}" type="number" name="${file}" min="${min}" max="${max}" placeholder="${schema.placeholder}"${preset} />`;
          }
          return html`<input class="${raw(dark)}" type="number" pattern="[0-9]*" oninput="this.value = this.value.replace(/[^0-9]/g, '');" name="${file}" min="${min}" max="${max}" placeholder="${schema.placeholder}"${preset} />`;
        })
      );
    case 'longtext': {
      const value = nextPrefill(cursor);
      return html`<textarea class="${raw(dark)}" name="${file}" rows="5" cols="33">${value ?? ''}</textarea>`;
    }
    case 'usageTitle': {
      const preset = nextPrefill(cursor) ?? schema.value;
      return html`<input id="usage-title-input_${argModalId}" class="${raw(dark)}" type="text" name="${file}" size="10"
        ${preset !== '' ? html`value="${preset}"` : raw('')}
    />`;
    }
    case 'text': {
      const preset = nextPrefill(cursor) ?? schema.value;
      return html`<input class="${raw(dark)}" type="text" name="${file}" size="10"
        ${preset !== '' ? html`value="${preset}"` : raw('')}
    />`;
    }
    case 'hidden':
      // Fixed carrier: consume the aligned position, render the declared value.
      nextPrefill(cursor);
      return html`<input class="invisible" type="text" size="10" value="${schema.value}" />`;
    case 'dropdown': {
      const base = queryBase(rctx);
      const effectiveFieldId = String(fieldId) === '' ? 1 : fieldId;
      const selected = nextPrefill(cursor);
      const options = schema.options.map((option, index) => {
        const optionId = index + 1;
        const optionName =
          option.label ?? text(`${base}__arg_${effectiveFieldId}_option_${optionId}_name`);
        return html`<option value="${option.id}"${selected !== undefined && option.id === selected ? raw(' selected') : raw('')}> ${optionName} </option>`;
      });
      return html`<select name="${file}">
        ${join(options)}
    </select>`;
    }
    case 'gpus': {
      const gpus = asObject(get(ctx.usage_example, 'gpus'));
      const selected = nextPrefill(cursor);
      const options = Object.entries(gpus).map(([gpu, usage]) => {
        const name = asString(asObject(usage)['name']);
        const label = name !== '' ? name : gpu;
        return html`<option value="${label}"${selected !== undefined && label === selected ? raw(' selected') : raw('')}> ${label} </option>`;
      });
      return html`<select name="${file}">
        ${join(options)}
    </select>`;
    }
    case 'diskLetter': {
      const disks = asObject(get(ctx.usage_example, 'disks'));
      const selected = nextPrefill(cursor) ?? 'C';
      const options = Object.entries(disks).map(
        ([disk]) =>
          html`<option value="${disk}"${disk === selected ? raw('\n                    selected') : raw('')}> ${disk} </option>`
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

/** One arg with its label/group chrome (shared add/edit template). */
export function argBranch(
  ctx: BootContext,
  rctx: ArgsRenderContext,
  arg: JsonObject,
  argIndex: number,
  argCounter: number,
  cursor?: PrefillCursor
): Html {
  const dark = ctx.dark_theme;
  const parsed = parseArg(arg);
  const base =
    rctx.subId !== 0
      ? `${catKey(rctx.category)}_${cmdKey(rctx.parentCommand)}_sub${rctx.subId}`
      : `${catKey(rctx.category)}_${cmdKey(rctx.command)}`;

  if (parsed.kind === 'input') {
    const argName = parsed.label ?? text(`${base}__arg_${argCounter}_name`);
    const folderForm = parsed.folderForm
      ? html`<div class="webdeck_foldername_div">
                      <form id="webdeck_foldername_form" novalidate>
                        <input class="${raw(dark)}" type="text" id="folderName_${rctx.argModalId}" name="folderName" placeholder="New folder name" />
                        <button id="submitButton_${rctx.argModalId}" type="submit">Create folder</button>
                      </form>
                    </div>`
      : raw('');
    return join([
      html`<div class="arg_container" ${raw(rctx.idAttr)}="${rctx.argModalId}" arg_id="${String(argIndex)}">
                  <label for="${argName}_${rctx.argModalId}">${argName}:</label>
                  ${renderField(rctx, parsed.field, argCounter, cursor)}
                </div>`,
      folderForm,
    ]);
  }
  if (parsed.kind === 'choice') {
    const override = cursor?.prefill.choices.get(argIndex);
    const choices = parsed.options.map((option, choiceIndex) => {
      const choiceId = choiceIndex + 1;
      const choiceName = option.label ?? text(`${base}__arg_${argCounter}_option_${choiceId}_name`);
      const selected = override !== undefined ? choiceIndex === override : option.checked;
      // NOTE: nested fields keep the legacy 0-based choice index as their
      // label id (only nested dropdowns consume it; none exist in
      // commands.json, so this preserves behavior exactly).
      // Only the selected pane consumes prefill (mirrors collection, which
      // skips hidden panes); the rest render defaults.
      const paneCursor = selected ? cursor : undefined;
      const items = join(
        option.fields.map((field) => renderField(rctx, field, choiceIndex, paneCursor))
      );
      return join([
        html`<div class="choice">
                      <input ${selected ? raw('checked') : raw('')} class="choice ${raw(dark)}" type="radio" name="choice" value="${String(choiceIndex)}" onchange="showArg_${rctx.argModalId}('${String(choiceIndex)}')" />
                      <label for="${choiceName}">${choiceName}</label>
                    </div>`,
        html`<div ${selected ? raw('') : raw('style="display: none;"')} class="arg_container" ${raw(rctx.idAttr)}="${rctx.argModalId}" arg_id="${String(choiceIndex)}">
                      ${items}
                    </div>`,
      ]);
    });
    return html`<div class="choices_ALL">
                  <label for="choice">${text('choose_option')} :</label><br />
                  ${join(choices)}
                </div>`;
  }
  if (parsed.kind === 'hidden') {
    return renderField(rctx, { kind: 'hidden', value: parsed.value }, argCounter, cursor);
  }
  return raw('');
}

export interface ArgsBlockOptions {
  ctx: BootContext;
  category: string;
  command: string;
  subId: number;
  parentCommand: string;
  commandValue: JsonObject;
  modalId: string;
  idAttr: ModalIdAttr;
  cursor?: PrefillCursor;
}

/**
 * Register one modal's choice-pane switcher (shared add/edit wire-up).
 * Shows the selected pane, hides its siblings; input args and the dev box
 * are untouched. (Legacy only hid — the `:not` filter excluded the target
 * pane, so switching choices blanked the form.)
 */
export function registerShowArg(modalId: string, idAttr: ModalIdAttr): void {
  (window as unknown as Record<string, unknown>)[`showArg_${modalId}`] = (argId: string) => {
    q(`div.choices_ALL div.arg_container[${idAttr}="${modalId}"][arg_id]`)
      .toArray()
      .forEach(function (element) {
        if (q(element).attr('arg_id') === argId) {
          q(element).css('display', 'block');
        } else {
          q(element).css('display', 'none');
        }
      });
  };
}

/** The `.args-container` block (shared add/edit template). */
export function renderArgsBlock(o: ArgsBlockOptions): Html {
  const dark = o.ctx.dark_theme;
  const args = asArray(o.commandValue['args']);
  let argCounter = 0;
  const blocks = args.map((argValue, argIndex) => {
    const arg = asObject(argValue);
    if (consumesArgNumber(arg)) argCounter++;
    const rctx: ArgsRenderContext = {
      ctx: o.ctx,
      category: o.category,
      command: o.command,
      subId: o.subId,
      parentCommand: o.parentCommand,
      argModalId: o.modalId,
      idAttr: o.idAttr,
      commandValue: o.commandValue,
    };
    return argBranch(o.ctx, rctx, arg, argIndex, argCounter, o.cursor);
  });
  // NOTE: inner whitespace is load-bearing — it keeps add-modal output
  // byte-identical with the pre-refactor template.
  return html`<div class="args-container ${raw(dark)}" ${raw(o.idAttr)}="${o.modalId}">
            ${join(blocks)}
          </div>`;
}
