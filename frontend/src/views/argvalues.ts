import { asArray, asObject, asString, rep, type JsonObject } from '../framework/types';
import type { ArgsPrefill } from './args';
import { parseArg, type ArgSchema, type ParsedArg } from './argschema';

/**
 * Saved-message → form resolution for the edit modal.
 *
 * A button stores its invocation as one `message` string
 * (`/key a`, `/exec type:file_path<|§|>C:\x.py`, ...). To render the same
 * arg form the add modal showed, the entry is located in `commands` by
 * command id and its `<|§|>`-split segments are aligned to the entry's
 * schema positions (the exact order `collectArgValues` collects):
 *
 * - hidden carriers must match their segment (this disambiguates entries
 *   that share a command id, e.g. the fifteen `/usage '` forms);
 * - visible fields consume one segment each, or render their default when
 *   the message is short;
 * - choice options resolve by trial: the first option whose fields align
 *   (hidden markers included) with the remaining segments wins;
 * - trailing unaligned segments fail the match — the modal then keeps its
 *   legacy form-less rendering instead of guessing.
 *
 * File inputs consume their segment but cannot display it (browsers forbid
 * prefilling them); the renderer stamps it as `data-preserved` so resaves
 * keep it (the `/exec` uploaded-file name lives in such a segment).
 */

export interface ResolvedCommand {
  category: string;
  command: string;
  subId: number;
  parentCommand: string;
  commandId: string;
  commandValue: JsonObject;
  prefill: ArgsPrefill;
}

interface Candidate extends ResolvedCommand {
  segments: string[];
}

/** Consume one field's positions; hidden carriers must match. */
function consumeField(field: ArgSchema, segments: string[], state: { pos: number; values: string[] }): boolean {
  const take = (): void => {
    if (state.pos < segments.length) {
      state.values.push(segments[state.pos]!);
      state.pos++;
    }
  };
  switch (field.kind) {
    case 'none':
      return true;
    case 'hidden':
      if (state.pos >= segments.length || segments[state.pos] !== field.value) return false;
      state.values.push(segments[state.pos]!);
      state.pos++;
      return true;
    case 'number':
      field.ranges.forEach(take);
      return true;
    case 'filetype':
      field.accepts.forEach(take);
      return true;
    case 'filepath':
      field.acceptLists.forEach(take);
      return true;
    default:
      take();
      return true;
  }
}

/**
 * Align parsed args to message segments. Returns the prefill on full
 * alignment (every hidden matched, every segment consumed).
 */
export function alignArgs(parsed: ParsedArg[], segments: string[]): ArgsPrefill | null {
  const values: string[] = [];
  const choices = new Map<number, number>();
  const state = { pos: 0, values };

  const alignFrom = (argIndex: number): boolean => {
    if (argIndex >= parsed.length) return state.pos === segments.length;
    const arg = parsed[argIndex]!;
    if (arg.kind === 'input') {
      return consumeField(arg.field, segments, state) && alignFrom(argIndex + 1);
    }
    if (arg.kind === 'hidden') {
      if (state.pos >= segments.length || segments[state.pos] !== arg.value) return false;
      state.values.push(segments[state.pos]!);
      state.pos++;
      return alignFrom(argIndex + 1);
    }
    if (arg.kind === 'choice') {
      for (let o = 0; o < arg.options.length; o++) {
        const savePos = state.pos;
        const saveLen = state.values.length;
        const fieldsOk = arg.options[o]!.fields.every((field) => consumeField(field, segments, state));
        if (fieldsOk && alignFrom(argIndex + 1)) {
          choices.set(argIndex, o);
          return true;
        }
        state.pos = savePos;
        state.values.length = saveLen;
      }
      return false;
    }
    return alignFrom(argIndex + 1);
  };

  if (!alignFrom(0)) return null;
  return { values, choices };
}

/** First input field kind of an entry (drives the URL tie-break). */
function firstFieldKind(entry: JsonObject): string {
  for (const argValue of asArray(entry['args'])) {
    const parsed = parseArg(asObject(argValue));
    if (parsed.kind === 'input') return parsed.field.kind;
  }
  return '';
}

function splitSegments(message: string, commandId: string): string[] {
  const rest = message.slice(commandId.length).trim();
  return rest === '' ? [] : rest.split('<|§|>');
}

/**
 * Locate the commands entry that produced `message`, with aligned prefill.
 * Returns null when nothing matches (unknown, hand-edited, or arg-less
 * messages render the legacy form-less edit modal).
 */
export function resolveButtonCommand(commands: JsonObject, message: string): ResolvedCommand | null {
  const trimmed = message.trim();
  if (trimmed === '') return null;
  const matches: Candidate[] = [];

  const consider = (
    category: string,
    command: string,
    subId: number,
    parentCommand: string,
    entry: JsonObject
  ): void => {
    const id = asString(entry['command']);
    if (id === '' || (trimmed !== id && !trimmed.startsWith(id + ' '))) return;
    const segments = splitSegments(trimmed, id);
    const parsed = asArray(entry['args']).map((argValue) => parseArg(asObject(argValue)));
    const prefill = alignArgs(parsed, segments);
    if (!prefill) return;
    matches.push({ category, command, subId, parentCommand, commandId: id, commandValue: entry, prefill, segments });
  };

  for (const [category, categoryValue] of Object.entries(commands)) {
    const catObj = asObject(categoryValue);
    for (const [command, commandValue] of Object.entries(catObj)) {
      if (command === 'CATEGORY-SETTINGS') continue;
      const cmdObj = asObject(commandValue);
      if (asString(cmdObj['TYPE']) === 'multiple') {
        asArray(cmdObj['commands']).forEach((subValue, subIndex) => {
          const subObj = asObject(subValue);
          consider(category, rep(asString(subObj['command']), '/', ''), subIndex + 1, command, subObj);
        });
      } else {
        consider(category, command, 0, '', cmdObj);
      }
    }
  }

  if (matches.length === 0) return null;
  if (matches.length === 1) return matches[0]!;
  // Duplicate command ids (e.g. `/start` = run-file vs website): a URL-like
  // first segment selects a URL-first form, anything else a non-URL form.
  const first = matches[0]!.segments[0] ?? '';
  const isUrl = /^https?:\/\//i.test(first);
  const urlFirst = matches.filter((m) => firstFieldKind(m.commandValue) === 'url');
  const nonUrl = matches.filter((m) => firstFieldKind(m.commandValue) !== 'url');
  if (isUrl && urlFirst.length > 0) return urlFirst[0]!;
  if (!isUrl && nonUrl.length > 0) return nonUrl[0]!;
  return matches[0]!;
}
