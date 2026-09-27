import { beforeEach, describe, expect, it } from 'vitest';
import { buildCommand } from './args';
import type { BootContext, JsonObject } from '../framework/types';
import { initI18n } from '../framework/i18n';
import { addArgsModal, type AddModalContext } from './addbutton';
import { editButtonModal, wireEditModal } from './editmodal';

const COMMANDS = {
  Text: {
    'Press a key': { command: '/key', args: [{ TYPE: 'input key' }], style: { image: 'key.png', image_size: '75%' } },
    Copy: {
      command: '/copy',
      args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
      style: { image: 'copy.png', image_size: '100%' },
    },
  },
  System: {
    'Execute python code': {
      command: '/exec',
      args: [
        {
          TYPE: 'choice',
          options: [
            { TYPE: 'multiple checked', items: [{ TYPE: 'text', value: 'type:uploaded_file' }, { TYPE: "input filetype['.py']" }] },
            { TYPE: 'multiple', items: [{ TYPE: 'text', value: 'type:file_path' }, { TYPE: "input filepath['.py']" }] },
            { TYPE: 'multiple', items: [{ TYPE: 'text', value: 'type:single_line' }, { TYPE: 'input text' }] },
          ],
        },
      ],
      style: { image: 'execpython.png', image_size: '70%' },
    },
  },
  Display: {
    CPU: {
      command: "/usage '",
      args: [
        { TYPE: 'input usage-title-text', value: 'CPU' },
        { TYPE: 'text', value: "' usage_dict['cpu']['usage_percent']" },
      ],
      style: { image: '', image_size: '' },
    },
  },
  Webdeck: {
    'Open a folder': { command: '/folder', args: [{ TYPE: 'input webdeck_foldername' }] },
  },
} as unknown as JsonObject;

const I18N = {
  configure_your_button: 'Configura tu botón',
  image: 'Imagen',
  image_size: 'Tamaño de la imagen',
  background_color: 'Color de fondo',
  background_color_hex: 'Color de fondo (HEX)',
  button_title: 'Título del botón',
  save: 'Guardar',
  choose_option: 'Elige opción',
  key_capture: 'Capturar',
  key_capture_prompt: 'Pulsa una tecla…',
  key_search_keys: 'Buscar teclas…',
  edit_command: 'Command',
  TEXT_press_a_key__arg_1_name: 'Tecla',
  TEXT_copy__arg_1_option_1_name: 'Copiar (ctrl+c)',
  TEXT_copy__arg_1_option_2_name: 'Copiar texto',
};

function testCtx(overrides: Partial<BootContext> = {}): BootContext {
  return {
    config: { front: { names_color: '', buttons: { folderA: [], folderB: [] } }, settings: {} },
    commands: COMMANDS,
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: false,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: '',
    ...overrides,
  };
}

function renderEdit(ctx: BootContext, modalId: string, message: string, extra: JsonObject = {}): string {
  const entry = { message, name: 'n', ...extra };
  return editButtonModal(ctx, 'folderA', 0, modalId, entry, message).value;
}

beforeEach(() => {
  initI18n({ ...I18N });
  document.body.innerHTML = '';
});

describe('edit modal arg form', () => {
  it('renders the resolved key field prefilled with its label', () => {
    const out = renderEdit(testCtx(), 'e0X0', '/key a');
    expect(out).toContain('class="args-container');
    expect(out).toContain('>Tecla:<');
    expect(out).toContain('id="key-input_e0X0"');
    expect(out).toContain('value="a"');
    expect(out).toContain('>Capturar</button>');
  });

  it('renders choice panes with the saved option selected', () => {
    const out = renderEdit(testCtx(), 'e0X0', '/copy hi');
    expect(out).toContain('>Copiar texto<');
    // Second pane visible, first hidden.
    expect(out).toMatch(/arg_id="1">\s*<input[^>]*value="hi"/);
    expect(out).toContain('style="display: none;" class="arg_container" edit_modal_ID="e0X0" arg_id="0"');
  });

  it('stays form-less for unresolvable messages', () => {
    const out = renderEdit(testCtx(), 'e0X0', '/whatever x');
    expect(out).not.toContain('args-container');
    expect(out).not.toContain('key-input_e0X0');
  });

  it('shows the dev command box only without an arg form', () => {
    const dev = testCtx({ config: { front: {}, settings: { dev_mode: true } } });
    expect(renderEdit(dev, 'e0X0', '/key a')).not.toContain('id="command_e0X0"');
    const legacy = renderEdit(dev, 'e0X0', '/whatever x');
    expect(legacy).toContain('id="command_e0X0"');
    expect(legacy).toContain('value="/whatever x"');
  });
});

describe('edit resave round-trip', () => {
  function mountEdit(message: string, ctx: BootContext = testCtx()): Element {
    document.body.innerHTML = renderEdit(ctx, 'e0X0', message);
    wireEditModal(ctx, 'e0X0', { message, name: 'n' });
    return document.querySelector('.args-container')!;
  }

  it('rebuilds unchanged messages verbatim', () => {
    expect(buildCommand('/key', mountEdit('/key a'))).toBe('/key a');
    expect(buildCommand('/copy', mountEdit('/copy hi'))).toBe('/copy hi');
    expect(buildCommand('/copy', mountEdit('/copy '))).toBe('/copy ');
    expect(
      buildCommand("/usage '", mountEdit("/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']"))
    ).toBe("/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']");
  });

  it('applies field edits to the rebuilt message', () => {
    const container = mountEdit('/key a');
    (document.querySelector('#key-input_e0X0') as HTMLInputElement).value = 'enter';
    expect(buildCommand('/key', container)).toBe('/key enter');
  });

  it('preserves upload filenames across resaves', () => {
    const message = '/exec type:uploaded_file<|§|>C:\\fakepath\\x.py';
    const container = mountEdit(message);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    expect(fileInput.getAttribute('data-preserved')).toBe('C:\\fakepath\\x.py');
    expect(buildCommand('/exec', container)).toBe(message);
  });

  it('switches choice panes through the shared switcher', () => {
    mountEdit('/copy hi');
    const show = (window as unknown as Record<string, (id: string) => void>)['showArg_e0X0']!;
    expect(typeof show).toBe('function');
    show('0');
    const panes = [...document.querySelectorAll('.choices_ALL .arg_container')] as HTMLElement[];
    expect(panes[0]!.style.display).toBe('block');
    expect(panes[1]!.style.display).toBe('none');
    expect(buildCommand('/copy', document.querySelector('.args-container')!)).toBe('/copy ');
  });

  it('binds the suffixed folder form without collisions', () => {
    document.body.innerHTML = renderEdit(testCtx(), 'e0X0', '/folder folderB');
    expect(document.querySelector('#submitButton_e0X0')).not.toBeNull();
    expect(document.querySelector('#folderName_e0X0')).not.toBeNull();
    expect(() => wireEditModal(testCtx(), 'e0X0', { message: '/folder folderB', name: 'n' })).not.toThrow();
    expect(buildCommand('/folder', document.querySelector('.args-container')!)).toBe('/folder folderB');
  });
});

describe('add/edit parity', () => {
  function argsInner(modalHtml: string, modalId: string): string {
    document.body.innerHTML = modalHtml;
    const inner = document.querySelector('.args-container')!.innerHTML;
    document.body.innerHTML = '';
    // split/join: String.replaceAll needs a newer lib than this target.
    const swap = (s: string, from: string, to: string): string => s.split(from).join(to);
    let out = swap(swap(inner, `_${modalId}`, '_ID'), modalId, 'ID');
    // Parsed HTML lowercases attribute names.
    out = swap(swap(out, 'arg_modal_id', 'modal_id'), 'edit_modal_id', 'modal_id');
    out = swap(swap(out, 'arg_modal_ID', 'modal_ID'), 'edit_modal_ID', 'modal_ID');
    return out;
  }

  function renderAdd(commandValue: JsonObject, category: string, command: string): string {
    const mctx: AddModalContext = {
      argModalId: '9X9',
      category,
      command,
      parentCommand: '',
      subId: 0,
      commandValue,
      commandId: String(commandValue['command'] ?? ''),
      buttonTitle: command,
    };
    return addArgsModal(testCtx(), mctx).value;
  }

  it.each([
    ['key field', 'Text', 'Press a key', '/key '],
    ['choice group', 'Text', 'Copy', '/copy '],
    ['usage form', 'Display', 'CPU', "/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']"],
    ['folder radios', 'Webdeck', 'Open a folder', '/folder '],
  ])('renders identical %s in both modals', (_label, category, command, message) => {
    const cat = (COMMANDS[category] ?? {}) as JsonObject;
    const commandValue = (cat[command] ?? {}) as JsonObject;
    const addInner = argsInner(renderAdd(commandValue, category, command), '9X9');
    const editInner = argsInner(renderEdit(testCtx(), 'e0X0', message), 'e0X0');
    expect(editInner).toBe(addInner);
  });
});
