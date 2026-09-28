import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { defineKeyField } from '../components/keyfield';
import { hasVisibleParams } from './args';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import type { AddModalContext } from './addbutton';
import AddArgsModal from './addbutton/AddArgsModal.svelte';
import AddBrowser from './addbutton/AddBrowser.svelte';

let apps: Record<string, never>[] = [];

afterEach(async () => {
  for (const app of apps) await unmount(app);
  apps = [];
  document.body.innerHTML = '';
});

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

async function renderModal(
  ctx: BootContext,
  category: string,
  command: string,
  commandValue: JsonObject
): Promise<HTMLElement> {
  defineKeyField();
  const host = document.createElement('div');
  document.body.appendChild(host);
  apps.push(
    mount(AddArgsModal, { target: host, props: { ctx, mctx: mctxFor(category, command, commandValue) } }) as unknown as Record<string, never>
  );
  await tick();
  return host;
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
  it('resolves the input label via the double-underscore key', async () => {
    initI18n({ ...editorKeys, TEXT_press_a_key__arg_1_name: 'Tecla' });
    const el = await renderModal(testCtx(), 'Text', 'Press a key', {
      command: '/key',
      args: [{ TYPE: 'input text' }],
      style: { image: 'key.png', image_size: '75%' },
    });
    expect(el.innerHTML).toContain('>Tecla:<');
    expect(el.innerHTML).not.toContain('TEXT_press_a_key_arg_1_name');
  });

  it('renders the key selector with its translated label', async () => {
    initI18n({
      ...editorKeys,
      TEXT_press_a_key__arg_1_name: 'Tecla',
      key_capture: 'Capturar',
      key_search_keys: 'Buscar teclas…',
    });
    const el = await renderModal(testCtx(), 'Text', 'Press a key', {
      command: '/key',
      args: [{ TYPE: 'input key' }],
      style: { image: 'key.png', image_size: '75%' },
    });
    expect(el.innerHTML).toContain('>Tecla:<');
    expect(el.innerHTML).not.toContain('TEXT_press_a_key_arg_1_name');
    // The key field itself is a custom-element island: assert the mounted DOM.
    expect(el.querySelector('.key-field')).not.toBeNull();
    expect(el.querySelector('#key-capture_9X9')?.textContent).toBe('Capturar');
    expect(el.querySelector('#key-list_9X9')?.getAttribute('placeholder')).toBe('Buscar teclas…');
  });

  it('resolves dropdown option labels with the 1-based arg number', async () => {
    initI18n({
      ...editorKeys,
      SYSTEM_screensaver__arg_1_name: 'Modo',
      SYSTEM_screensaver__arg_1_option_1_name: 'Nada',
      SYSTEM_screensaver__arg_1_option_2_name: 'Completo',
      SYSTEM_screensaver__arg_1_option_3_name: 'Apagado',
    });
    const el = await renderModal(testCtx(), 'System', 'ScreenSaver', {
      command: '/screensaver',
      args: [{ TYPE: 'input dropdown', options: [{ ID: 'NONE' }, { ID: 'full' }, { ID: 'off' }] }],
    });
    expect(el.innerHTML).toContain('>Modo:<');
    expect(el.innerHTML).toContain('>Nada</option>');
    expect(el.innerHTML).toContain('>Completo</option>');
    expect(el.innerHTML).toContain('>Apagado</option>');
    expect(el.innerHTML).not.toContain('SYSTEM_screensaver_arg_');
  });

  it('prefers inline plugin labels over i18n lookups', async () => {
    initI18n({ ...editorKeys, choose_option: 'Choose' });
    const el = await renderModal(testCtx(), 'Calc', 'add', {
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
    });
    expect(el.innerHTML).toContain('>First:<');
    expect(el.innerHTML).toContain('>Nothing<');
    expect(el.innerHTML).toContain('>Something<');
    expect(el.innerHTML).toContain('>Mode:<');
    expect(el.innerHTML).toContain('>Ex</option>');
    expect(el.innerHTML).not.toContain('CALC_add_arg_');
  });

  it('shows the inline plugin description in the command browser', async () => {
    initI18n({ ...editorKeys, choose_option: 'Choose' });
    const ctx = testCtx();
    ctx.commands = {
      Calc: {
        add: { command: '/add', description: 'adds two numbers', args: [] },
      },
    };
    const host = document.createElement('div');
    document.body.appendChild(host);
    apps.push(mount(AddBrowser, { target: host, props: { ctx } }) as unknown as Record<string, never>);
    await tick();
    expect(host.innerHTML).toContain('adds two numbers');
    expect(host.innerHTML).toContain('add');
    // Leaf rows keep their opener contract: id + modal-id + command tag.
    const btn = host.querySelector('#open-button-0X0') as HTMLElement;
    expect(btn?.getAttribute('arg_modal_ID')).toBe('0X0');
    expect(btn?.getAttribute('dropdown-commandTag')).toBe('add');
    expect(host.querySelector('#modal-container-0X0')).not.toBeNull();
  });

  it('resolves choice option labels via the double-underscore key', async () => {
    initI18n({
      ...editorKeys,
      choose_option: 'Elige opción',
      TEXT_copy__arg_1_option_1_name: 'Copiar (ctrl+c)',
      TEXT_copy__arg_1_option_2_name: 'Copiar texto',
    });
    const el = await renderModal(testCtx(), 'Text', 'Copy', {
      command: '/copy',
      args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
      style: { image: 'copy.png', image_size: '100%' },
    });
    expect(el.innerHTML).toContain('>Copiar (ctrl+c)<');
    expect(el.innerHTML).toContain('>Copiar texto<');
    expect(el.innerHTML).not.toContain('TEXT_copy_arg_');
  });
});

describe('hasVisibleParams', () => {
  it('ignores empty, hidden-only, and unknown args', () => {
    expect(hasVisibleParams({ command: '/fullscreen', args: [] })).toBe(false);
    expect(hasVisibleParams({ command: '/x', args: [{ TYPE: 'text', value: 'v' }] })).toBe(false);
    expect(hasVisibleParams({ command: '/x', args: [{ TYPE: 'bogus' }] })).toBe(false);
    expect(hasVisibleParams({ command: '/x' })).toBe(false);
  });

  it('detects input and choice args', () => {
    expect(hasVisibleParams({ command: '/x', args: [{ TYPE: 'input text' }] })).toBe(true);
    expect(
      hasVisibleParams({ command: '/x', args: [{ TYPE: 'choice', options: [] }] })
    ).toBe(true);
    expect(
      hasVisibleParams({
        command: '/x',
        args: [{ TYPE: 'text', value: 'v' }, { TYPE: 'input key' }],
      })
    ).toBe(true);
  });

  it('hides the Parameters tab when nothing visible would render', async () => {
    initI18n({ ...editorKeys });
    const el = await renderModal(testCtx(), 'Webdeck', 'Fullscreen', {
      command: '/fullscreen',
      args: [],
      style: { image: 'fullscreen2.png', image_size: '50%' },
    });
    const tabs = el.querySelectorAll('[role="tab"]');
    expect(tabs).toHaveLength(1);
    expect(tabs[0]?.textContent?.trim()).toBe('Appearance');
    expect(el.querySelector('#add-9X9-pane-args')).toBeNull();
    expect(el.querySelector('#add-9X9-pane-look')?.hasAttribute('hidden')).toBe(false);
  });
});
