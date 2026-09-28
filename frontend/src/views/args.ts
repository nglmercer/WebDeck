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
 * Args data model: one parsed field with its render-order prefill resolved.
 * Built by a single traversal (`argsData`) so the string renderer and
 * `ArgsBlock.svelte` share numbering/cursor semantics exactly.
 */
export type FieldData =
  | { kind: 'none' }
  | { kind: 'foldername'; folders: string[]; checked: string | undefined }
  | { kind: 'audioUpload'; preserved: string | undefined }
  | { kind: 'filetype'; inputs: Array<{ accepts: string; preserved: string | undefined }> }
  | { kind: 'filepath'; inputs: Array<{ filetypes: string; value: string | undefined }> }
  | { kind: 'filePicker'; value: string | undefined }
  | { kind: 'folderPicker'; value: string | undefined }
  | { kind: 'url'; value: string | undefined }
  | { kind: 'key'; value: string }
  | {
      kind: 'number';
      inputs: Array<{ min: string; max: string; value: string | undefined }>;
      placeholder: string;
    }
  | { kind: 'longtext'; value: string | undefined }
  | { kind: 'usageTitle'; preset: string }
  | { kind: 'text'; preset: string }
  | { kind: 'hidden'; value: string }
  | { kind: 'dropdown'; options: Array<{ id: string; label: string; selected: boolean }> }
  | { kind: 'gpus'; options: Array<{ key: string; label: string; selected: boolean }> }
  | { kind: 'diskLetter'; options: Array<{ disk: string; selected: boolean }> };

export type BranchData =
  | { kind: 'input'; argIndex: number; label: string; folderForm: boolean; field: FieldData }
  | {
      kind: 'choice';
      options: Array<{ choiceIndex: number; name: string; selected: boolean; fields: FieldData[] }>;
    }
  | { kind: 'hidden'; field: FieldData }
  | { kind: 'none' };

export interface ArgsData {
  dark: string;
  modalId: string;
  idAttr: ModalIdAttr;
  branches: BranchData[];
}

function queryBase(rctx: ArgsRenderContext): string {
  const cat = rctx.category.replace(/ /g, '').toUpperCase();
  let cmd = rctx.command.replace(/ /g, '_').replace(/'/g, '').toLowerCase();
  if (rctx.subId !== 0) {
    cmd = rctx.parentCommand.replace(/ /g, '_').replace(/'/g, '').toLowerCase() + '_sub' + rctx.subId;
  }
  return `${cat}_${cmd}`;
}

function branchBase(rctx: ArgsRenderContext): string {
  return rctx.subId !== 0
    ? `${catKey(rctx.category)}_${cmdKey(rctx.parentCommand)}_sub${rctx.subId}`
    : `${catKey(rctx.category)}_${cmdKey(rctx.command)}`;
}

/** Resolve one parsed field's runtime values (consumes prefill in order). */
export function fieldData(
  rctx: ArgsRenderContext,
  schema: ArgSchema,
  fieldId: string | number,
  cursor?: PrefillCursor
): FieldData {
  const { ctx } = rctx;
  switch (schema.kind) {
    case 'none':
      return { kind: 'none' };
    case 'foldername':
      return {
        kind: 'foldername',
        folders: Object.keys(asObject(get(ctx.config, 'front', 'buttons'))),
        checked: nextPrefill(cursor),
      };
    case 'audioUpload':
      return { kind: 'audioUpload', preserved: nextPrefill(cursor) };
    case 'filetype':
      return {
        kind: 'filetype',
        inputs: schema.accepts.map((filetypes) => ({
          accepts: filetypes.join(', '),
          preserved: nextPrefill(cursor),
        })),
      };
    case 'filepath':
      return {
        kind: 'filepath',
        inputs: schema.acceptLists.map((filetypes) => ({
          filetypes: filetypes.join('_'),
          value: nextPrefill(cursor),
        })),
      };
    case 'filePicker':
      return { kind: 'filePicker', value: nextPrefill(cursor) };
    case 'folderPicker':
      return { kind: 'folderPicker', value: nextPrefill(cursor) };
    case 'url':
      return { kind: 'url', value: nextPrefill(cursor) };
    case 'key':
      return { kind: 'key', value: nextPrefill(cursor) ?? schema.value };
    case 'number':
      return {
        kind: 'number',
        inputs: schema.ranges.map(({ min, max }) => ({ min, max, value: nextPrefill(cursor) })),
        placeholder: schema.placeholder,
      };
    case 'longtext':
      return { kind: 'longtext', value: nextPrefill(cursor) };
    case 'usageTitle':
      return { kind: 'usageTitle', preset: nextPrefill(cursor) ?? schema.value };
    case 'text':
      return { kind: 'text', preset: nextPrefill(cursor) ?? schema.value };
    case 'hidden':
      // Fixed carrier: consume the aligned position, render the declared value.
      nextPrefill(cursor);
      return { kind: 'hidden', value: schema.value };
    case 'dropdown': {
      const base = queryBase(rctx);
      const effectiveFieldId = String(fieldId) === '' ? 1 : fieldId;
      const selected = nextPrefill(cursor);
      return {
        kind: 'dropdown',
        options: schema.options.map((option, index) => ({
          id: option.id,
          label: option.label ?? text(`${base}__arg_${effectiveFieldId}_option_${index + 1}_name`),
          selected: selected !== undefined && option.id === selected,
        })),
      };
    }
    case 'gpus': {
      const gpus = asObject(get(ctx.usage_example, 'gpus'));
      const selected = nextPrefill(cursor);
      return {
        kind: 'gpus',
        options: Object.entries(gpus).map(([gpu, usage]) => {
          // The option value is embedded verbatim as the `usage_dict['gpus']`
          // key in the tile message, so it must be the dict key (`GPU1`,
          // `defaultGPU`, …): names never match a key, and names with spaces
          // also break the dotted tile-path lookup — either way the tile
          // sticks at `-`. The name stays as the visible label.
          const name = asString(asObject(usage)['name']);
          const label = name !== '' ? name : gpu;
          return { key: gpu, label, selected: selected !== undefined && (gpu === selected || label === selected) };
        }),
      };
    }
    case 'diskLetter': {
      const disks = asObject(get(ctx.usage_example, 'disks'));
      const selected = nextPrefill(cursor) ?? 'C';
      return {
        kind: 'diskLetter',
        options: Object.entries(disks).map(([disk]) => ({ disk, selected: disk === selected })),
      };
    }
  }
}

/** Resolve one arg's branch chrome (consumes prefill in collection order). */
export function branchData(
  rctx: ArgsRenderContext,
  arg: JsonObject,
  argIndex: number,
  argCounter: number,
  cursor?: PrefillCursor
): BranchData {
  const parsed = parseArg(arg);
  const base = branchBase(rctx);
  if (parsed.kind === 'input') {
    return {
      kind: 'input',
      argIndex,
      label: parsed.label ?? text(`${base}__arg_${argCounter}_name`),
      folderForm: parsed.folderForm,
      field: fieldData(rctx, parsed.field, argCounter, cursor),
    };
  }
  if (parsed.kind === 'choice') {
    const override = cursor?.prefill.choices.get(argIndex);
    return {
      kind: 'choice',
      options: parsed.options.map((option, choiceIndex) => {
        const selected = override !== undefined ? choiceIndex === override : option.checked;
        // NOTE: nested fields keep the legacy 0-based choice index as their
        // label id (only nested dropdowns consume it; none exist in
        // commands.json, so this preserves behavior exactly).
        // Only the selected pane consumes prefill (mirrors collection, which
        // skips hidden panes); the rest render defaults.
        const paneCursor = selected ? cursor : undefined;
        return {
          choiceIndex,
          name: option.label ?? text(`${base}__arg_${argCounter}_option_${choiceIndex + 1}_name`),
          selected,
          fields: option.fields.map((field) => fieldData(rctx, field, choiceIndex, paneCursor)),
        };
      }),
    };
  }
  if (parsed.kind === 'hidden') {
    return {
      kind: 'hidden',
      field: fieldData(rctx, { kind: 'hidden', value: parsed.value }, argCounter, cursor),
    };
  }
  return { kind: 'none' };
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

/** Resolve a whole args block (single numbering/cursor traversal). */
export function argsData(o: ArgsBlockOptions): ArgsData {
  const args = asArray(o.commandValue['args']);
  let argCounter = 0;
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
  const branches = args.map((argValue, argIndex) => {
    const arg = asObject(argValue);
    if (consumesArgNumber(arg)) argCounter++;
    return branchData(rctx, arg, argIndex, argCounter, o.cursor);
  });
  return { dark: o.ctx.dark_theme, modalId: o.modalId, idAttr: o.idAttr, branches };
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

