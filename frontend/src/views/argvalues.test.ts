import { describe, expect, it } from 'vitest';
import type { JsonObject } from '../framework/types';
import { alignArgs, resolveButtonCommand } from './argvalues';
import { parseArg } from './argschema';

function align(rawArgs: JsonObject[], segments: string[]) {
  return alignArgs(
    rawArgs.map((arg) => parseArg(arg)),
    segments
  );
}

const KEY = { command: '/key', args: [{ TYPE: 'input key' }] };
const COPY = {
  command: '/copy',
  args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
};
const EXEC = {
  command: '/exec',
  args: [
    {
      TYPE: 'choice',
      options: [
        { TYPE: 'multiple checked', items: [{ TYPE: 'text', value: 'type:uploaded_file' }, { TYPE: "input filetype['.rhai']" }] },
        { TYPE: 'multiple', items: [{ TYPE: 'text', value: 'type:file_path' }, { TYPE: "input filepath['.rhai']" }] },
        { TYPE: 'multiple', items: [{ TYPE: 'text', value: 'type:single_line' }, { TYPE: 'input text' }] },
      ],
    },
  ],
};
const CPU = {
  command: "/usage '",
  args: [{ TYPE: 'input usage-title-text', value: 'CPU' }, { TYPE: 'text', value: "' usage_dict['cpu']['usage_percent']" }],
};
const DISKS = {
  command: "/usage '",
  args: [
    { TYPE: 'input usage-title-text', value: 'Disk' },
    { TYPE: 'text', value: "' usage_dict['disks']['" },
    { TYPE: 'input disk-letter' },
    { TYPE: 'text', value: "']['usage_percent']" },
  ],
};

describe('alignArgs', () => {
  it('aligns visible fields positionally', () => {
    expect(align(KEY.args as JsonObject[], ['a'])).toEqual({ values: ['a'], choices: new Map() });
  });

  it('resolves choices by trial, including empty selections', () => {
    expect(align(COPY.args as JsonObject[], ['hi'])).toEqual({
      values: ['hi'],
      choices: new Map([[0, 1]]),
    });
    expect(align(COPY.args as JsonObject[], [])).toEqual({ values: [], choices: new Map([[0, 0]]) });
  });

  it('resolves multi-item choices by hidden markers', () => {
    expect(align(EXEC.args as JsonObject[], ['type:uploaded_file', 'C:\\fakepath\\x.rhai'])).toEqual({
      values: ['type:uploaded_file', 'C:\\fakepath\\x.rhai'],
      choices: new Map([[0, 0]]),
    });
    expect(align(EXEC.args as JsonObject[], ['type:file_path', 'C:\\x.rhai'])).toEqual({
      values: ['type:file_path', 'C:\\x.rhai'],
      choices: new Map([[0, 1]]),
    });
    expect(align(EXEC.args as JsonObject[], ['type:single_line', 'print(1)'])).toEqual({
      values: ['type:single_line', 'print(1)'],
      choices: new Map([[0, 2]]),
    });
  });

  it('rejects hidden mismatches and trailing segments', () => {
    expect(align(EXEC.args as JsonObject[], ['type:bogus', 'x'])).toBeNull();
    expect(align(KEY.args as JsonObject[], ['a', 'b'])).toBeNull();
  });

  it('tolerates short messages with defaults', () => {
    expect(align(KEY.args as JsonObject[], [])).toEqual({ values: [], choices: new Map() });
  });
});

describe('resolveButtonCommand', () => {
  const commands = {
    Text: { 'Press a key': KEY, Copy: COPY },
    System: {
      'Execute script code': EXEC,
      Open: { command: '/start', args: [{ TYPE: 'input file' }] },
      'Open a website': { command: '/start', args: [{ TYPE: 'input url' }] },
    },
    Display: { CPU, Disks: { TYPE: 'multiple', commands: [DISKS] } },
  } as unknown as JsonObject;

  it('resolves exact and valued messages', () => {
    const key = resolveButtonCommand(commands, '/key a')!;
    expect(key).not.toBeNull();
    expect(key.category).toBe('Text');
    expect(key.commandId).toBe('/key');
    expect(key.prefill.values).toEqual(['a']);

    const bare = resolveButtonCommand(commands, '/copy ')!;
    expect(bare.prefill.choices.get(0)).toBe(0);
  });

  it('rejects unknown, empty, and prefix-colliding messages', () => {
    expect(resolveButtonCommand(commands, '/nope x')).toBeNull();
    expect(resolveButtonCommand(commands, '')).toBeNull();
    expect(resolveButtonCommand(commands, '   ')).toBeNull();
    // `/keys x` must not match `/key` (space boundary).
    expect(resolveButtonCommand(commands, '/keys x')).toBeNull();
  });

  it('disambiguates shared ids by hidden markers', () => {
    const cpu = resolveButtonCommand(commands, "/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']")!;
    expect(cpu.command).toBe('CPU');
    expect(cpu.prefill.values[0]).toBe('CPU');

    const disks = resolveButtonCommand(
      commands,
      "/usage ' Disk<|§|>' usage_dict['disks']['<|§|>D<|§|>']['usage_percent']"
    )!;
    expect(disks.subId).toBe(1);
    expect(disks.parentCommand).toBe('Disks');
    expect(disks.prefill.values).toEqual(['Disk', "' usage_dict['disks']['", 'D', "']['usage_percent']"]);
  });

  it('sniffs URLs apart from file paths for /start', () => {
    const web = resolveButtonCommand(commands, '/start https://example.com')!;
    expect(web.command).toBe('Open a website');
    const file = resolveButtonCommand(commands, '/start C:\\x.exe')!;
    expect(file.command).toBe('Open');
  });

  it('resolves fakepath-carrying exec messages', () => {
    const exec = resolveButtonCommand(commands, '/exec type:uploaded_file<|§|>C:\\fakepath\\x.rhai')!;
    expect(exec.command).toBe('Execute script code');
    expect(exec.prefill.choices.get(0)).toBe(0);
    expect(exec.prefill.values).toEqual(['type:uploaded_file', 'C:\\fakepath\\x.rhai']);
  });
});
