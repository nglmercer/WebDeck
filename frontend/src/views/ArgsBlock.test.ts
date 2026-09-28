import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
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
  idAttr: ModalIdAttr = 'arg_modal_ID',
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
    expect(container.getAttribute('arg_modal_ID')).toBe('9X9');
    expect(container.querySelector('label')?.textContent).toBe('Tecla:');
    expect(container.querySelector('input[type="text"]')).not.toBeNull();
  });

  it('renders the key island with its prefill', async () => {
    const el = await render(
      options(
        { args: [{ TYPE: 'input key' }] },
        'e0X0',
        'edit_modal_ID',
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

  it('switches choice panes through the showArg global', async () => {
    const el = await render(
      options({ args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }] })
    );
    const panes = el.querySelectorAll('.choices_ALL .arg_container');
    expect((panes[0] as HTMLElement).style.display).toBe('');
    expect((panes[1] as HTMLElement).style.display).toBe('none');
    const radios = el.querySelectorAll('.choices_ALL input.choice');
    expect(radios[1]?.getAttribute('onchange')).toBe("showArg_9X9('1')");
    // happy-dom doesn't compile content-attribute handlers; invoke the
    // registered global directly (browsers run it via the attr above).
    (window as unknown as Record<string, (argId: string) => void>)['showArg_9X9']?.('1');
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
        'arg_modal_ID',
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
});
