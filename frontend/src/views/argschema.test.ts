import { describe, expect, it } from 'vitest';
import {
  consumesArgNumber,
  evalList,
  parseArg,
  parseChoiceOption,
  parseField,
} from './argschema';

describe('evalList', () => {
  it('parses quoted python-list literals', () => {
    expect(evalList("['0','100']")).toEqual(['0', '100']);
    expect(evalList('["a", "b"]')).toEqual(['a', 'b']);
  });

  it('falls back to comma splitting', () => {
    expect(evalList('[1,2]')).toEqual(['1', '2']);
    expect(evalList('[]')).toEqual([]);
  });
});

describe('parseField', () => {
  it('parses text inputs with and without preset values', () => {
    expect(parseField({ TYPE: 'input text' })).toEqual({ kind: 'text', value: '' });
    expect(parseField({ TYPE: 'input text', value: 'hi' })).toEqual({ kind: 'text', value: 'hi' });
  });

  it('parses usage-title inputs', () => {
    expect(parseField({ TYPE: 'input usage-title-text', value: 'CPU' })).toEqual({
      kind: 'usageTitle',
      value: 'CPU',
    });
  });

  it('parses hidden text carriers', () => {
    expect(parseField({ TYPE: 'text', value: 'type:file_path' })).toEqual({
      kind: 'hidden',
      value: 'type:file_path',
    });
    // Pure text without a value renders nothing.
    expect(parseField({ TYPE: 'text' })).toEqual({ kind: 'none' });
  });

  it('parses longtext and textarea', () => {
    expect(parseField({ TYPE: 'input longtext' })).toEqual({ kind: 'longtext' });
    expect(parseField({ TYPE: 'input textarea' })).toEqual({ kind: 'longtext' });
  });

  it('parses url inputs', () => {
    expect(parseField({ TYPE: 'input url' })).toEqual({ kind: 'url' });
  });

  it('parses headers editors', () => {
    expect(parseField({ TYPE: 'input headers' })).toEqual({ kind: 'headers' });
  });

  it('parses key selectors without stealing neighboring kinds', () => {
    expect(parseField({ TYPE: 'input key' })).toEqual({ kind: 'key', value: '' });
    expect(parseField({ TYPE: 'input key', value: 'enter' })).toEqual({ kind: 'key', value: 'enter' });
    expect(parseField({ TYPE: 'input text' }).kind).toBe('text');
    expect(parseField({ TYPE: 'input file' }).kind).toBe('filePicker');
  });

  it('parses number ranges with placeholders', () => {
    expect(parseField({ TYPE: "input number['1','100']", placeholder: '50' })).toEqual({
      kind: 'number',
      ranges: [{ min: '1', max: '100' }],
      placeholder: '50',
    });
    expect(parseField({ TYPE: "input number['0','100']" })).toEqual({
      kind: 'number',
      ranges: [{ min: '0', max: '100' }],
      placeholder: '',
    });
    // Negative bounds are preserved verbatim for the renderer.
    expect(parseField({ TYPE: "input number['-5','5']" })).toEqual({
      kind: 'number',
      ranges: [{ min: '-5', max: '5' }],
      placeholder: '',
    });
  });

  it('parses file inputs with accept lists', () => {
    expect(parseField({ TYPE: "input filetype['.exe']" })).toEqual({
      kind: 'filetype',
      accepts: [['.exe']],
    });
    expect(parseField({ TYPE: "input filepath['.py']" })).toEqual({
      kind: 'filepath',
      acceptLists: [['.py']],
    });
    expect(parseField({ TYPE: 'input file' })).toEqual({ kind: 'filePicker' });
    expect(parseField({ TYPE: 'input folderpath' })).toEqual({ kind: 'folderPicker' });
  });

  it('prefers filetype/filepath over the bare file branch', () => {
    expect(parseField({ TYPE: "input filetype['.mp3']" }).kind).toBe('filetype');
    expect(parseField({ TYPE: "input filepath['.py']" }).kind).toBe('filepath');
  });

  it('parses the legacy soundboard audio upload', () => {
    expect(parseField({ TYPE: 'input path-soundboard-audio' })).toEqual({ kind: 'audioUpload' });
  });

  it('parses folder selectors', () => {
    expect(parseField({ TYPE: 'input webdeck_foldername' })).toEqual({ kind: 'foldername' });
    expect(parseField({ TYPE: 'input disk-letter' })).toEqual({ kind: 'diskLetter' });
    expect(parseField({ TYPE: 'input available_gpus' })).toEqual({ kind: 'gpus' });
  });

  it('parses dropdown option ids', () => {
    expect(
      parseField({ TYPE: 'input dropdown', options: [{ ID: 'NONE' }, { ID: 'full' }] })
    ).toEqual({ kind: 'dropdown', options: [{ id: 'NONE' }, { id: 'full' }] });
  });

  it('maps NONE and bare multiple to none', () => {
    expect(parseField({ TYPE: 'NONE' })).toEqual({ kind: 'none' });
    expect(parseField({ TYPE: 'NONE checked' })).toEqual({ kind: 'none' });
    expect(parseField({ TYPE: 'multiple' })).toEqual({ kind: 'none' });
  });

  it('maps unknown declarations to none', () => {
    expect(parseField({ TYPE: 'input teleporter' })).toEqual({ kind: 'none' });
    expect(parseField({})).toEqual({ kind: 'none' });
  });
});

describe('inline labels', () => {
  it('carries verbatim labels for plugin args', () => {
    expect(parseArg({ TYPE: 'input text', label: 'First' })).toEqual({
      kind: 'input',
      folderForm: false,
      field: { kind: 'text', value: '' },
      label: 'First',
    });
    expect(parseChoiceOption({ TYPE: 'input text', label: 'Only' })).toEqual({
      checked: false,
      fields: [{ kind: 'text', value: '' }],
      label: 'Only',
    });
    expect(
      parseField({ TYPE: 'input dropdown', options: [{ ID: 'a', label: 'Alpha' }, { ID: 'b' }] })
    ).toEqual({
      kind: 'dropdown',
      options: [{ id: 'a', label: 'Alpha' }, { id: 'b' }],
    });
  });

  it('omits empty labels so i18n stays in charge', () => {
    expect(parseArg({ TYPE: 'input text', label: '' })).toEqual({
      kind: 'input',
      folderForm: false,
      field: { kind: 'text', value: '' },
    });
  });
});

describe('parseChoiceOption', () => {
  it('detects checked options and parses single fields', () => {
    expect(parseChoiceOption({ TYPE: 'NONE checked' })).toEqual({ checked: true, fields: [{ kind: 'none' }] });
    expect(parseChoiceOption({ TYPE: 'input text' })).toEqual({
      checked: false,
      fields: [{ kind: 'text', value: '' }],
    });
  });

  it('parses multiple-item groups', () => {
    expect(
      parseChoiceOption({
        TYPE: 'multiple checked',
        items: [{ TYPE: 'text', value: 'type:uploaded_file' }, { TYPE: "input filetype['.py']" }],
      })
    ).toEqual({
      checked: true,
      fields: [
        { kind: 'hidden', value: 'type:uploaded_file' },
        { kind: 'filetype', accepts: [['.py']] },
      ],
    });
  });
});

describe('parseArg', () => {
  it('parses labeled inputs and flags the folder form', () => {
    expect(parseArg({ TYPE: 'input text' })).toEqual({
      kind: 'input',
      folderForm: false,
      field: { kind: 'text', value: '' },
    });
    expect(parseArg({ TYPE: 'input webdeck_foldername' })).toEqual({
      kind: 'input',
      folderForm: true,
      field: { kind: 'foldername' },
    });
  });

  it('parses choice groups', () => {
    expect(
      parseArg({ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] })
    ).toEqual({
      kind: 'choice',
      options: [
        { checked: true, fields: [{ kind: 'none' }] },
        { checked: false, fields: [{ kind: 'text', value: '' }] },
      ],
    });
  });

  it('parses top-level hidden carriers and fallthroughs', () => {
    expect(parseArg({ TYPE: 'text', value: 'v' })).toEqual({ kind: 'hidden', value: 'v' });
    expect(parseArg({ TYPE: 'text' })).toEqual({ kind: 'none' });
    expect(parseArg({ TYPE: 'multiple' })).toEqual({ kind: 'none' });
    expect(parseArg({ TYPE: 'whatever' })).toEqual({ kind: 'none' });
  });
});

describe('consumesArgNumber', () => {
  it('reserves numbers for every arg except pure text', () => {
    expect(consumesArgNumber({ TYPE: 'text' })).toBe(false);
    expect(consumesArgNumber({ TYPE: 'input text' })).toBe(true);
    expect(consumesArgNumber({ TYPE: 'choice' })).toBe(true);
    expect(consumesArgNumber({ TYPE: 'NONE' })).toBe(true);
  });
});
