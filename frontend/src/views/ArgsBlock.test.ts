import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineKeyField } from '../components/keyfield';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import ArgsBlock from './ArgsBlock.svelte';
import {
  buildCommand,
  type ArgsBlockOptions,
  type ArgsPrefill,
  type ModalIdAttr,
} from './args';

function testCtx(): BootContext {
  initI18n({
    choose_option: 'Choose',
    select_your_file: 'Select file',
    no_file_chosen: 'No file',
    key_capture: 'Capture',
    key_capture_prompt: 'Press…',
    key_search_keys: 'Search…',
    TEXT_press_a_key__arg_1_name: 'Tecla',
  });
  return {
    config: { front: { names_color: '', buttons: { folderA: [], folderB: [] } }, settings: {} },
    commands: {},
    versions: {},
    random_bg: '',
    usage_example: { gpus: { GPU1: { name: 'RTX 4090' } }, disks: { C: {}, D: {} } },
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

function options(
  commandValue: JsonObject,
  modalId = '9X9',
  idAttr: ModalIdAttr = 'data-arg-modal-id',
  prefill?: ArgsPrefill
): ArgsBlockOptions {
  return {
    ctx: testCtx(),
    category: 'Text',
    command: 'Press a key',
    subId: 0,
    parentCommand: '',
    commandValue,
    modalId,
    idAttr,
    ...(prefill !== undefined ? { cursor: { prefill, pos: 0 } } : {}),
  };
}

describe('ArgsBlock', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  beforeEach(() => {
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
    vi.unstubAllGlobals();
  });

  async function render(o: ArgsBlockOptions): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(ArgsBlock, {
      target: host,
      props: {
        ctx: o.ctx,
        category: o.category,
        command: o.command,
        subId: o.subId,
        parentCommand: o.parentCommand,
        commandValue: o.commandValue,
        modalId: o.modalId,
        idAttr: o.idAttr,
        prefill: o.cursor?.prefill,
      },
    }) as unknown as Record<string, never>;
    await tick();
    return host;
  }

  it('renders labeled input fields', async () => {
    const el = await render(options({ args: [{ TYPE: 'input text' }] }));
    const container = el.querySelector('.args-container') as HTMLElement;
    expect(container.getAttribute('data-arg-modal-id')).toBe('9X9');
    expect(container.querySelector('label')?.textContent).toBe('Tecla:');
    expect(container.querySelector('input[type="text"]')).not.toBeNull();
  });

  it('renders the key island with its prefill', async () => {
    const el = await render(
      options(
        { args: [{ TYPE: 'input key' }] },
        'e0X0',
        'data-edit-modal-id',
        { values: ['a'], choices: new Map() }
      )
    );
    expect(el.querySelector('key-field')).not.toBeNull();
    expect((el.querySelector('#key-input_e0X0') as HTMLInputElement).value).toBe('a');
  });

  it('scrubs non-digits in positive number fields only', async () => {
    const el = await render(
      options({ args: [{ TYPE: "input number['1','100']", placeholder: '50' }] })
    );
    const input = el.querySelector('input[type="number"]') as HTMLInputElement;
    input.value = '7x5';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(input.value).toBe('75');
  });

  it('switches choice panes through component-owned change callbacks', async () => {
    const el = await render(
      options({ args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }] })
    );
    const panes = el.querySelectorAll('.choices_ALL .arg_container');
    expect((panes[0] as HTMLElement).style.display).toBe('');
    expect((panes[1] as HTMLElement).style.display).toBe('none');
    const radios = el.querySelectorAll('.choices_ALL input.choice');
    expect(radios[1]?.getAttribute('onchange')).toBeNull();
    radios[1]!.dispatchEvent(new Event('change', { bubbles: true }));
    await tick();
    expect((panes[0] as HTMLElement).style.display).toBe('none');
    expect((panes[1] as HTMLElement).style.display).toBe('block');
  });

  it('selects dropdown, gpu, and disk options', async () => {
    const el = await render(
      options(
        {
          args: [
            { TYPE: 'input dropdown', options: [{ ID: 'NONE' }, { ID: 'full' }] },
            { TYPE: 'input available_gpus' },
            { TYPE: 'input disk-letter' },
          ],
        },
        '9X9',
        'data-arg-modal-id',
        { values: ['full', 'GPU1'], choices: new Map() }
      )
    );
    const selects = el.querySelectorAll('.args-container select');
    expect((selects[0] as HTMLSelectElement).value).toBe('full');
    expect((selects[1] as HTMLSelectElement).value).toBe('GPU1');
    // Disk letters default to C without prefill.
    expect((selects[2] as HTMLSelectElement).value).toBe('C');
  });

  it('collects the default choice as an empty first pane', async () => {
    const el = await render(
      options({
        command: '/copy',
        args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
      })
    );
    expect(buildCommand('/copy', el.querySelector('.args-container') as Element)).toBe('/copy ');
  });

  it('fills the row input from the folder picker', async () => {
    const el = await render(options({ args: [{ TYPE: 'input folderpath' }] }));
    const fetchMock = vi.fn(async () => ({ ok: true, text: async () => '/home/u/Music' }));
    vi.stubGlobal('fetch', fetchMock);

    (el.querySelector('button.folderpath') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect((el.querySelector('input.folderpath') as HTMLInputElement).value).toBe('/home/u/Music')
    );
    const [folderUrl] = fetchMock.mock.calls[0] as unknown as [string];
    expect(folderUrl).toContain('upload_folderpath');
  });

  it('fills the row input from the file picker with filetypes', async () => {
    const el = await render(options({ args: [{ TYPE: "input filepath['.py']" }] }));
    const fetchMock = vi.fn(async () => ({ ok: true, text: async () => '/home/u/x.py' }));
    vi.stubGlobal('fetch', fetchMock);

    (el.querySelector('button.filepath') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect((el.querySelector('input.filepath') as HTMLInputElement).value).toBe('/home/u/x.py')
    );
    const [url] = fetchMock.mock.calls[0] as unknown as [string];
    expect(url).toContain('upload_filepath');
    expect(url).toContain('filetypes=');
  });

  it('scopes picker fills to the clicked row', async () => {
    const el = await render(
      options({ args: [{ TYPE: 'input folderpath' }, { TYPE: 'input folderpath' }] })
    );
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, text: async () => '/picked' })));

    const buttons = el.querySelectorAll('button.folderpath');
    (buttons[0] as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect((el.querySelectorAll('input.folderpath')[0] as HTMLInputElement).value).toBe('/picked')
    );
    expect((el.querySelectorAll('input.folderpath')[1] as HTMLInputElement).value).toBe('');
  });

  it('uploads audio files on selection', async () => {
    const el = await render(options({ args: [{ TYPE: 'input path-soundboard-audio' }] }));
    const fetchMock = vi.fn(async () => ({ ok: true }));
    vi.stubGlobal('fetch', fetchMock);

    const input = el.querySelector('.audio-input') as HTMLInputElement;
    Object.defineProperty(input, 'files', {
      value: [new File(['x'], 's.mp3', { type: 'audio/mpeg' })],
      configurable: true,
    });
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const [uploadUrl] = fetchMock.mock.calls[0] as unknown as [string];
    expect(uploadUrl).toBe('/upload_file');
  });
});
