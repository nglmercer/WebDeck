import { asArray, asObject, asString, type JsonObject } from '../framework/types';

/**
 * Schema layer for add-button arg forms.
 *
 * `commands.json` declares args as free-form `TYPE` strings (a port of the
 * old `args.jinja` conventions: `input text`, `input number['1','100']`,
 * `choice`, ...). This module parses each declaration once into a typed
 * {@link ParsedArg} so renderers switch on `kind` instead of re-matching
 * substrings. Unknown declarations parse to `none` (renders nothing), which
 * matches the legacy fallthrough.
 *
 * Precedence mirrors the legacy substring chain exactly: where two branches
 * could match one `TYPE` string (e.g. `file` vs `filetype`), the legacy
 * order decides. Keep {@link parseField} and {@link parseArg} in that order.
 */

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

/** One parsed field: the `argField` side of an arg (no label chrome). */
export type ArgSchema =
  | { kind: 'none' }
  | { kind: 'hidden'; value: string }
  | { kind: 'text'; value: string }
  | { kind: 'usageTitle'; value: string }
  | { kind: 'longtext' }
  | { kind: 'url' }
  | { kind: 'headers' }
  | { kind: 'key'; value: string }
  | { kind: 'number'; ranges: Array<{ min: string; max: string }>; placeholder: string }
  | { kind: 'filetype'; accepts: string[][] }
  | { kind: 'audioUpload' }
  | { kind: 'filepath'; acceptLists: string[][] }
  | { kind: 'filePicker' }
  | { kind: 'folderPicker' }
  | { kind: 'foldername' }
  | { kind: 'dropdown'; options: Array<{ id: string; label?: string }> }
  | { kind: 'diskLetter' }
  | { kind: 'gpus' };

/** One parsed `choice` option: radio + the field(s) it reveals. */
export interface ChoiceOption {
  checked: boolean;
  fields: ArgSchema[];
  /**
   * Inline label shown verbatim (plugin args have no `.lang` entries).
   * Present only when the declaration sets a non-empty `label`.
   */
  label?: string;
}

/** One parsed top-level arg: the `argBranch` side (label/group chrome). */
export type ParsedArg =
  | { kind: 'input'; folderForm: boolean; field: ArgSchema; label?: string }
  | { kind: 'choice'; options: ChoiceOption[] }
  | { kind: 'hidden'; value: string }
  | { kind: 'none' };

/** Read an inline `label` override (verbatim display text, see {@link ChoiceOption}). */
function inlineLabel(obj: JsonObject): string | undefined {
  const label = asString(obj['label']);
  return label !== '' ? label : undefined;
}

/** Parse one space-separated `prefix[...]` segment list (number/filetype/filepath). */
function evalSegments(type: string, prefix: string): string[][] {
  const out: string[][] = [];
  for (const item of type.split(' ')) {
    if (item.startsWith(prefix)) {
      out.push(evalList(item.replace(prefix, '')));
    }
  }
  return out;
}

/**
 * Parse one field declaration (legacy `argField` dispatch).
 * Pure shape parse: no context needed.
 */
export function parseField(arg: JsonObject): ArgSchema {
  const type = asString(arg['TYPE']);
  if (type.includes('webdeck_foldername')) {
    return { kind: 'foldername' };
  }
  if (type.includes('path-soundboard-audio')) {
    return { kind: 'audioUpload' };
  }
  if (type.includes('filetype')) {
    return { kind: 'filetype', accepts: evalSegments(type, 'filetype') };
  }
  if (type.includes('filepath')) {
    return { kind: 'filepath', acceptLists: evalSegments(type, 'filepath') };
  }
  if (type.includes('file')) {
    return { kind: 'filePicker' };
  }
  if (type.includes('folderpath')) {
    return { kind: 'folderPicker' };
  }
  if (type.includes('url')) {
    return { kind: 'url' };
  }
  // New (non-legacy) kind: no precedence interaction — no legacy TYPE
  // contains 'headers', and 'headers' matches no earlier branch.
  if (type.includes('headers')) {
    return { kind: 'headers' };
  }
  if (type.includes('key')) {
    return { kind: 'key', value: asString(arg['value']) };
  }
  if (type.includes('number')) {
    const ranges = evalSegments(type, 'number').map((list) => ({
      min: list[0] ?? '',
      max: list[1] ?? '',
    }));
    return { kind: 'number', ranges, placeholder: asString(arg['placeholder']) };
  }
  if (type.includes('longtext') || type.includes('textarea')) {
    return { kind: 'longtext' };
  }
  if (type.includes('usage-title-text') && type.includes('input')) {
    return { kind: 'usageTitle', value: asString(arg['value']) };
  }
  if (type.includes('text') && type.includes('input')) {
    return { kind: 'text', value: asString(arg['value']) };
  }
  if (type.includes('text') && asString(arg['value']) !== '') {
    return { kind: 'hidden', value: asString(arg['value']) };
  }
  if (type.includes('dropdown')) {
    const options = asArray(arg['options']).map((option) => {
      const obj = asObject(option);
      const label = inlineLabel(obj);
      return label === undefined ? { id: asString(obj['ID']) } : { id: asString(obj['ID']), label };
    });
    return { kind: 'dropdown', options };
  }
  if (type.includes('available_gpus')) {
    return { kind: 'gpus' };
  }
  if (type.includes('disk-letter')) {
    return { kind: 'diskLetter' };
  }
  return { kind: 'none' };
}

/** Parse one `choice` option (legacy choice branch in `argBranch`). */
export function parseChoiceOption(choice: JsonObject): ChoiceOption {
  const type = asString(choice['TYPE']);
  const checked = type.includes('checked');
  const label = inlineLabel(choice);
  const fields = type.includes('multiple')
    ? asArray(choice['items']).map((item) => parseField(asObject(item)))
    : [parseField(choice)];
  return label === undefined ? { checked, fields } : { checked, fields, label };
}

/**
 * Parse one top-level arg (legacy `argBranch` dispatch).
 * Returns the chrome kind; labels/numbering stay with the renderer.
 */
export function parseArg(arg: JsonObject): ParsedArg {
  const type = asString(arg['TYPE']);
  if (type.includes('input')) {
    const folderForm = type.includes('webdeck_foldername');
    const field = parseField(arg);
    const label = inlineLabel(arg);
    return label === undefined
      ? { kind: 'input', folderForm, field }
      : { kind: 'input', folderForm, field, label };
  }
  if (type.includes('choice')) {
    return {
      kind: 'choice',
      options: asArray(arg['options']).map((option) => parseChoiceOption(asObject(option))),
    };
  }
  if (type.includes('text') && asString(arg['value']) !== '') {
    return { kind: 'hidden', value: asString(arg['value']) };
  }
  return { kind: 'none' };
}

/**
 * Legacy numbering: every arg except pure `text` consumes one label number.
 * (Pure `text` args are invisible value carriers.)
 */
export function consumesArgNumber(arg: JsonObject): boolean {
  return asString(arg['TYPE']) !== 'text';
}
