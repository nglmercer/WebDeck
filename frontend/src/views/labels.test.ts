import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { collectAddModals } from './addbutton';
import { argsData, type BranchData, type FieldData } from './args';

// Repo root, resolved from this file (Vite denies `?raw` imports above root).
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

function readRepo(relative: string): string {
  return readFileSync(join(ROOT, relative), 'utf-8');
}

const commandsJson = JSON.parse(readRepo('webdeck/commands.json')) as JsonObject;

/**
 * Label-coverage audit: resolve every add-button args block from the real
 * `commands.json` and fail on any raw `*_arg_N…` key leaking into the
 * labels. Arg labels have no display fallback (unlike button titles), so a
 * missing `.lang` entry — or a misbuilt query — shows the raw key to users.
 *
 * English gaps fail; other languages only report (console) so one missing
 * translation can't block unrelated work.
 */

/** Mirror of the Rust `.lang` parser (`load_lang_file`): same skips, same split. */
function parseLang(source: string): Record<string, string> {
  const dict: Record<string, string> = {};
  for (const rawLine of source.split('\n')) {
    const line = rawLine.trim();
    if (line === '' || line.startsWith('//') || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq === -1) throw new Error(`Invalid line format: ${line}`);
    dict[line.slice(0, eq).trim()] = line.slice(eq + 1).trim();
  }
  return dict;
}

const LANG_CODES = ['de_DE', 'en_US', 'es_ES', 'fr_FR', 'ko_KR', 'pl_PL', 'ru_RU'];

function langSource(code: string): string {
  return readRepo(`webdeck/translations/${code}.lang`);
}

/** Raw arg/option query keys (`CAT_cmd__arg_1_name`, `…_option_2_name`). */
const RAW_ARG_KEY = /\b[A-Z][^\s<"']*?__?arg_\d[^\s<"]*/g;

function auditCtx(): BootContext {
  return {
    config: {
      front: { names_color: '', buttons: { folderA: [], folderB: [] } },
      settings: {},
    },
    commands: commandsJson as unknown as JsonObject,
    versions: {},
    random_bg: '',
    usage_example: { gpus: { '0': { name: 'RTX 4090' } }, disks: { C: {}, D: {} } },
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: 'dark-theme',
  };
}

interface Leak {
  modal: string;
  keys: string[];
}

/** Every resolved display string one field can emit. */
function fieldLabels(field: FieldData): string[] {
  switch (field.kind) {
    case 'dropdown':
      return field.options.map((o) => o.label);
    case 'gpus':
      return field.options.map((o) => o.label);
    case 'number':
      return [field.placeholder];
    default:
      return [];
  }
}

function branchLabels(branch: BranchData): string[] {
  if (branch.kind === 'input') return [branch.label, ...fieldLabels(branch.field)];
  if (branch.kind === 'choice') {
    return branch.options.flatMap((o) => [o.name, ...o.fields.flatMap(fieldLabels)]);
  }
  if (branch.kind === 'hidden') return fieldLabels(branch.field);
  return [];
}

function audit(dict: Record<string, string>): Leak[] {
  initI18n(dict);
  const ctx = auditCtx();
  const modals = collectAddModals(ctx);
  expect(modals.length).toBeGreaterThan(0);
  const leaks: Leak[] = [];
  for (const mctx of modals) {
    // Every `__arg_N` query resolves inside `argsData` (labels, option
    // names, dropdown labels); scanning the resolved strings covers the
    // same surface the old markup audit did, without rendering.
    const data = argsData({
      ctx,
      category: mctx.category,
      command: mctx.command,
      subId: mctx.subId,
      parentCommand: mctx.parentCommand,
      commandValue: mctx.commandValue,
      modalId: mctx.argModalId,
      idAttr: 'arg_modal_ID',
    });
    const keys = [
      ...new Set(data.branches.flatMap(branchLabels).flatMap((s) => s.match(RAW_ARG_KEY) ?? [])),
    ];
    if (keys.length > 0) {
      leaks.push({ modal: `${mctx.argModalId} ${mctx.category} / ${mctx.command}`, keys });
    }
  }
  return leaks;
}

function formatLeaks(leaks: Leak[]): string {
  return leaks.map((l) => `  ${l.modal}: ${l.keys.join(', ')}`).join('\n');
}

describe('arg label coverage', () => {
  it('has no raw arg keys in English modals', () => {
    const dict = parseLang(langSource('en_US'));
    expect(Object.keys(dict).length).toBeGreaterThan(400);
    const leaks = audit(dict);
    expect(leaks, `raw arg keys leaked:\n${formatLeaks(leaks)}`).toEqual([]);
  });

  it('reports per-language gaps without failing', () => {
    const reports: string[] = [];
    for (const code of LANG_CODES) {
      if (code === 'en_US') continue;
      const leaks = audit(parseLang(langSource(code)));
      if (leaks.length > 0) {
        const keys = leaks.flatMap((l) => l.keys);
        reports.push(
          `${code}: ${keys.length} raw key(s) in ${leaks.length} modal(s):\n${formatLeaks(leaks)}`
        );
      }
    }
    if (reports.length > 0) {
      console.log(`[labels] translation gaps (non-blocking):\n${reports.join('\n')}`);
    }
  });
});
