import { describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { addArgsModal, addBrowserView, type AddModalContext } from './addbutton';

function testCtx(): BootContext {
  return {
    config: { front: { names_color: '' }, settings: {} },
    commands: {},
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
  };
}

function mctxFor(category: string, command: string, commandValue: JsonObject): AddModalContext {
  return {
    argModalId: '9X9',
    category,
    command,
    parentCommand: '',
    subId: 0,
    commandValue,
    commandId: String(commandValue['command'] ?? ''),
    buttonTitle: command,
  };
}

const editorKeys = {
  configure_your_button: 'Configura tu botón',
  image: 'Imagen',
  image_size: 'Tamaño de la imagen',
  background_color: 'Color de fondo',
  background_color_hex: 'Color de fondo (HEX)',
  button_title: 'Título del botón',
  save: 'Guardar',
};

describe('arg labels', () => {
  it('resolves the input label via the double-underscore key', () => {
    initI18n({ ...editorKeys, TEXT_press_a_key__arg_1_name: 'Tecla' });
    const out = addArgsModal(
      testCtx(),
      mctxFor('Text', 'Press a key', {
        command: '/key',
        args: [{ TYPE: 'input text' }],
        style: { image: 'key.png', image_size: '75%' },
      })
    ).value;
    expect(out).toContain('>Tecla:<');
    expect(out).not.toContain('TEXT_press_a_key_arg_1_name');
  });

  it('renders the key selector with its translated label', () => {
    initI18n({
      ...editorKeys,
      TEXT_press_a_key__arg_1_name: 'Tecla',
      key_capture: 'Capturar',
      key_search_keys: 'Buscar teclas…',
    });
    const out = addArgsModal(
      testCtx(),
      mctxFor('Text', 'Press a key', {
        command: '/key',
        args: [{ TYPE: 'input key' }],
        style: { image: 'key.png', image_size: '75%' },
      })
    ).value;
    expect(out).toContain('>Tecla:<');
    expect(out).toContain('class="key-field"');
    expect(out).toContain('>Capturar</button>');
    expect(out).toContain('placeholder="Buscar teclas…"');
    expect(out).not.toContain('TEXT_press_a_key_arg_1_name');
  });

  it('resolves dropdown option labels with the 1-based arg number', () => {
    initI18n({
      ...editorKeys,
      SYSTEM_screensaver__arg_1_name: 'Modo',
      SYSTEM_screensaver__arg_1_option_1_name: 'Nada',
      SYSTEM_screensaver__arg_1_option_2_name: 'Completo',
      SYSTEM_screensaver__arg_1_option_3_name: 'Apagado',
    });
    const out = addArgsModal(
      testCtx(),
      mctxFor('System', 'ScreenSaver', {
        command: '/screensaver',
        args: [{ TYPE: 'input dropdown', options: [{ ID: 'NONE' }, { ID: 'full' }, { ID: 'off' }] }],
      })
    ).value;
    expect(out).toContain('>Modo:<');
    expect(out).toContain('> Nada </option>');
    expect(out).toContain('> Completo </option>');
    expect(out).toContain('> Apagado </option>');
    expect(out).not.toContain('SYSTEM_screensaver_arg_');
  });

  it('prefers inline plugin labels over i18n lookups', () => {
    initI18n({ ...editorKeys, choose_option: 'Choose' });
    const out = addArgsModal(
      testCtx(),
      mctxFor('Calc', 'add', {
        command: '/add',
        args: [
          { TYPE: "input number['1','100']", label: 'First', placeholder: '1' },
          {
            TYPE: 'choice',
            options: [
              { TYPE: 'NONE checked', label: 'Nothing' },
              { TYPE: 'input text', label: 'Something' },
            ],
          },
          { TYPE: 'input dropdown', label: 'Mode', options: [{ ID: 'x', label: 'Ex' }] },
        ],
      })
    ).value;
    expect(out).toContain('>First:<');
    expect(out).toContain('>Nothing<');
    expect(out).toContain('>Something<');
    expect(out).toContain('>Mode:<');
    expect(out).toContain('> Ex </option>');
    expect(out).not.toContain('CALC_add_arg_');
  });

  it('shows the inline plugin description in the command browser', () => {
    initI18n({ ...editorKeys, choose_option: 'Choose' });
    const ctx = testCtx();
    ctx.commands = {
      Calc: {
        add: { command: '/add', description: 'adds two numbers', args: [] },
      },
    };
    const out = addBrowserView(ctx).value;
    expect(out).toContain('adds two numbers');
    expect(out).toContain('add');
  });

  it('resolves choice option labels via the double-underscore key', () => {
    initI18n({
      ...editorKeys,
      choose_option: 'Elige opción',
      TEXT_copy__arg_1_option_1_name: 'Copiar (ctrl+c)',
      TEXT_copy__arg_1_option_2_name: 'Copiar texto',
    });
    const out = addArgsModal(
      testCtx(),
      mctxFor('Text', 'Copy', {
        command: '/copy',
        args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
        style: { image: 'copy.png', image_size: '100%' },
      })
    ).value;
    expect(out).toContain('>Copiar (ctrl+c)<');
    expect(out).toContain('>Copiar texto<');
    expect(out).not.toContain('TEXT_copy_arg_');
  });
});
