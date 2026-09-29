import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineKeyField } from '../components/keyfield';
import { buildCommand, collectArgValues } from './args';
import type { BootContext, JsonObject } from '../framework/types';
import { initI18n } from '../framework/i18n';
import { type AddModalContext } from './addbutton';
import AddArgsModal from './addbutton/AddArgsModal.svelte';
import EditModal from './EditModal.svelte';
import { wireEditModal } from './editmodal';

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
    'Execute script code': {
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
      style: { image: 'execscript.svg', image_size: '70%' },
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
    Fullscreen: {
      command: '/fullscreen',
      args: [],
      style: { image: 'fullscreen2.png', image_size: '50%' },
    },
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

describe('edit modal arg form', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  beforeEach(() => {
    initI18n({ ...I18N });
    defineKeyField();
    document.body.innerHTML = '';
  });

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  async function mountEdit(
    ctx: BootContext,
    modalId: string,
    message: string,
    extra: JsonObject = {}
  ): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    const entry = { message, name: 'n', ...extra };
    app = mount(EditModal, {
      target: host,
      props: { ctx, editModalId: modalId, entry, message },
    }) as unknown as Record<string, never>;
    await tick();
    return host;
  }

  it('renders the resolved key field prefilled with its label', async () => {
    const el = await mountEdit(testCtx(), 'e0X0', '/key a');
    expect(el.querySelector('.args-container')).not.toBeNull();
    expect(el.querySelector('.args-container label')?.textContent).toBe('Tecla:');
    expect((el.querySelector('#key-input_e0X0') as HTMLInputElement).value).toBe('a');
    expect(el.querySelector('#key-capture_e0X0')?.textContent).toBe('Capturar');
  });

  it('renders choice panes with the saved option selected', async () => {
    const el = await mountEdit(testCtx(), 'e0X0', '/copy hi');
    expect(el.innerHTML).toContain('>Copiar texto<');
    const panes = [...el.querySelectorAll('.choices_ALL .arg_container')] as HTMLElement[];
    // Second pane visible with the saved value, first hidden.
    expect(panes[1]!.style.display).toBe('');
    expect((panes[1]!.querySelector('input[type="text"]') as HTMLInputElement).value).toBe('hi');
    expect(panes[0]!.style.display).toBe('none');
    expect(panes[0]!.getAttribute('arg_id')).toBe('0');
  });

  it('hides the Parameters tab for resolved commands without visible params', async () => {
    const el = await mountEdit(testCtx(), 'e0X0', '/fullscreen');
    const tabs = el.querySelectorAll('[role="tab"]');
    expect(tabs).toHaveLength(1);
    expect(tabs[0]?.textContent?.trim()).toBe('Appearance');
    expect(el.querySelector('#edit-e0X0-pane-args')).toBeNull();
    expect(el.querySelector('#edit-e0X0-pane-look')?.hasAttribute('hidden')).toBe(false);
    // The command badge still identifies the button action.
    expect(el.querySelector('.wd2-cmd')?.textContent).toBe('/fullscreen');
  });

  it('stays form-less for unresolvable messages', async () => {
    const el = await mountEdit(testCtx(), 'e0X0', '/whatever x');
    expect(el.querySelector('.args-container')).toBeNull();
    expect(el.querySelector('#key-input_e0X0')).toBeNull();
    // Style block + save still render.
    expect(el.querySelector('.editorStyle')).not.toBeNull();
    expect(el.querySelector('#e0X0_submit')).not.toBeNull();
  });

  it('shows the dev command box only without an arg form', async () => {
    const dev = testCtx({ config: { front: {}, settings: { dev_mode: true } } });
    const withForm = await mountEdit(dev, 'e0X0', '/key a');
    expect(withForm.querySelector('#command_e0X0')).toBeNull();
    if (app) await unmount(app);
    app = null;
    host?.remove();
    const legacy = await mountEdit(dev, 'e0X0', '/whatever x');
    expect((legacy.querySelector('#command_e0X0') as HTMLInputElement).value).toBe('/whatever x');
  });
});

describe('edit resave round-trip', () => {
  beforeEach(() => {
    initI18n({ ...I18N });
    defineKeyField();
    document.body.innerHTML = '';
  });

  async function mountEdit(message: string, ctx: BootContext = testCtx()): Promise<Element> {
    document.body.innerHTML = '';
    const host = document.createElement('div');
    document.body.appendChild(host);
    mount(EditModal, {
      target: host,
      props: { ctx, editModalId: 'e0X0', entry: { message, name: 'n' }, message },
    });
    await tick();
    wireEditModal(ctx, 'e0X0', { message, name: 'n' });
    return document.querySelector('.args-container')!;
  }

  it('rebuilds unchanged messages verbatim', async () => {
    expect(buildCommand('/key', await mountEdit('/key a'))).toBe('/key a');
    expect(buildCommand('/copy', await mountEdit('/copy hi'))).toBe('/copy hi');
    expect(buildCommand('/copy', await mountEdit('/copy '))).toBe('/copy ');
    expect(
      buildCommand("/usage '", await mountEdit("/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']"))
    ).toBe("/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']");
  });

  it('applies field edits to the rebuilt message', async () => {
    const container = await mountEdit('/key a');
    (document.querySelector('#key-input_e0X0') as HTMLInputElement).value = 'enter';
    expect(buildCommand('/key', container)).toBe('/key enter');
  });

  it('preserves upload filenames across resaves', async () => {
    const message = '/exec type:uploaded_file<|§|>C:\\fakepath\\x.rhai';
    const container = await mountEdit(message);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    expect(fileInput.getAttribute('data-preserved')).toBe('C:\\fakepath\\x.rhai');
    expect(buildCommand('/exec', container)).toBe(message);
  });

  it('switches choice panes through the shared switcher', async () => {
    await mountEdit('/copy hi');
    const show = (window as unknown as Record<string, (id: string) => void>)['showArg_e0X0']!;
    expect(typeof show).toBe('function');
    show('0');
    const panes = [...document.querySelectorAll('.choices_ALL .arg_container')] as HTMLElement[];
    expect(panes[0]!.style.display).toBe('block');
    expect(panes[1]!.style.display).toBe('none');
    expect(buildCommand('/copy', document.querySelector('.args-container')!)).toBe('/copy ');
  });

  it('binds the suffixed folder form without collisions', async () => {
    await mountEdit('/folder folderB');
    expect(document.querySelector('#submitButton_e0X0')).not.toBeNull();
    expect(document.querySelector('#folderName_e0X0')).not.toBeNull();
    expect(() => wireEditModal(testCtx(), 'e0X0', { message: '/folder folderB', name: 'n' })).not.toThrow();
    expect(buildCommand('/folder', document.querySelector('.args-container')!)).toBe('/folder folderB');
  });
});

describe('add/edit parity', () => {
  beforeEach(() => {
    initI18n({ ...I18N });
    defineKeyField();
    document.body.innerHTML = '';
  });

  async function renderAdd(
    commandValue: JsonObject,
    category: string,
    command: string
  ): Promise<{ host: HTMLElement; app: Record<string, never> }> {
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
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(AddArgsModal, { target: host, props: { ctx: testCtx(), mctx } }) as unknown as Record<string, never>;
    await tick();
    return { host, app };
  }

  it.each([
    ['key field', 'Text', 'Press a key', '/key ', '/key'],
    ['choice group', 'Text', 'Copy', '/copy ', '/copy'],
    ['usage form', 'Display', 'CPU', "/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']", "/usage '"],
    ['folder radios', 'Webdeck', 'Open a folder', '/folder ', '/folder'],
  ])('collects identical %s in both modals', async (_label, category, command, message, commandId) => {
    const cat = (COMMANDS[category] ?? {}) as JsonObject;
    const commandValue = (cat[command] ?? {}) as JsonObject;
    // Add side: Svelte renderer, mounted.
    const { host: addHost, app: addApp } = await renderAdd(commandValue, category, command);
    // Edit side: Svelte renderer, mounted.
    const editHost = document.createElement('div');
    document.body.appendChild(editHost);
    const editApp = mount(EditModal, {
      target: editHost,
      props: {
        ctx: testCtx(),
        editModalId: 'e0X0',
        entry: { message, name: 'n' },
        message,
      },
    });
    await tick();
    const addValues = collectArgValues(addHost.querySelector('.args-container')!);
    const editValues = collectArgValues(editHost.querySelector('.args-container')!);
    expect(editValues).toEqual(addValues);
    expect(`${commandId} ${editValues.join('<|§|>')}`).toBe(`${commandId} ${addValues.join('<|§|>')}`);
    await unmount(editApp as unknown as Record<string, never>);
    await unmount(addApp);
    addHost.remove();
    editHost.remove();
  });
});

describe('edit submit coalescing', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = '';
  });

  async function mountWiredEdit(): Promise<{ app: Record<string, never>; host: HTMLElement }> {
    initI18n({ ...I18N });
    defineKeyField();
    const ctx = testCtx();
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(EditModal, {
      target: host,
      props: {
        ctx,
        editModalId: 'e0X0',
        entry: { message: '/copy hi', name: 'n' },
        message: '/copy hi',
      },
    }) as unknown as Record<string, never>;
    await tick();
    wireEditModal(ctx, 'e0X0', { message: '/copy hi', name: 'n' });
    return { app, host };
  }

  function saveCalls(fetchMock: ReturnType<typeof vi.fn>): unknown[][] {
    return fetchMock.mock.calls.filter(([url]) => String(url).includes('save_single_button'));
  }

  /** Dismiss the queued alert dialog (keeps the dialog queue usable). */
  async function dismissAlert(): Promise<void> {
    await vi.waitFor(() => expect(document.querySelector('[data-testid="alert-ok"]')).not.toBeNull());
    (document.querySelector('[data-testid="alert-ok"]') as HTMLElement).click();
    await vi.waitFor(() => expect(document.querySelector('[role="alertdialog"]')).toBeNull());
  }

  it('sends one save for rapid double submits', async () => {
    const { app, host } = await mountWiredEdit();
    try {
      const fetchMock = vi.fn(async (_url: string) => ({
        // Failure response: the save is still sent (what this counts),
        // but no modal-hide timers outlive the test environment.
        ok: true,
        json: async () => ({ success: false }),
      }));
      vi.stubGlobal('fetch', fetchMock);

      const submit = document.querySelector('#e0X0_submit') as HTMLInputElement;
      submit.click();
      submit.click();
      await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(1));
      // Settled saves stay at one (no delayed duplicate).
      await new Promise((resolve) => setTimeout(resolve, 50));
      expect(saveCalls(fetchMock)).toHaveLength(1);
      await dismissAlert();
    } finally {
      await unmount(app);
      host.remove();
    }
  });

  it('disables the submit control in flight and allows retry after failure', async () => {
    const { app, host } = await mountWiredEdit();
    try {
      let rejectSave!: (error: unknown) => void;
      const gate = new Promise<never>((_resolve, reject) => {
        rejectSave = reject;
      });
      const fetchMock = vi.fn((_url: string) => gate);
      vi.stubGlobal('fetch', fetchMock);

      const submit = document.querySelector('#e0X0_submit') as HTMLInputElement;
      submit.click();
      await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(1));
      expect(submit.disabled).toBe(true);

      rejectSave(new Error('boom'));
      await vi.waitFor(() => expect(submit.disabled).toBe(false));
      // The failure stays visible (no false success).
      await vi.waitFor(() =>
        expect(document.querySelector('#wd-dialog-message')?.textContent).toBe('boom')
      );
      await dismissAlert();

      submit.click();
      await vi.waitFor(() => expect(saveCalls(fetchMock)).toHaveLength(2));
      await dismissAlert();
    } finally {
      await unmount(app);
      host.remove();
    }
  });
});
