import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { defineKeyField } from '../components/keyfield';
import { initI18n } from '../framework/i18n';
import type { BootContext, JsonObject } from '../framework/types';
import { getCommand, type AddModalContext } from './addbutton';
import AddArgsModal from './addbutton/AddArgsModal.svelte';

/**
 * Serialization goldens: `getCommand` must keep producing the exact
 * `<|§|>`-joined command strings the backend splits. Fixtures mirror real
 * `webdeck/commands.json` entries. Labels are irrelevant here (empty dict),
 * only collected values matter.
 */

const MODAL_ID = '7X7';

function testCtx(overrides: Partial<BootContext> = {}): BootContext {
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
    ...overrides,
  };
}

let apps: Record<string, never>[] = [];

afterEach(async () => {
  for (const app of apps) await unmount(app);
  apps = [];
});

async function render(
  ctx: BootContext,
  category: string,
  command: string,
  commandValue: JsonObject
): Promise<void> {
  const mctx: AddModalContext = {
    argModalId: MODAL_ID,
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
  apps.push(mount(AddArgsModal, { target: host, props: { ctx, mctx } }) as unknown as Record<string, never>);
  await tick();
}

function fields(): HTMLElement[] {
  return [...document.querySelectorAll(`form[data-arg-modal-id="${MODAL_ID}"] .args-container input, form[data-arg-modal-id="${MODAL_ID}"] .args-container select, form[data-arg-modal-id="${MODAL_ID}"] .args-container textarea`)] as HTMLElement[];
}

function setValue(el: HTMLElement, value: string): void {
  (el as HTMLInputElement).value = value;
}

beforeEach(() => {
  initI18n({});
  defineKeyField();
  document.body.innerHTML = '';
});

describe('getCommand goldens', () => {
  it('serializes a text arg (Text / Press a key)', async () => {
    await render(testCtx(), 'Text', 'Press a key', {
      command: '/key',
      args: [{ TYPE: 'input text' }],
      style: { image: 'key.png', image_size: '75%' },
    });
    setValue(fields()[0]!, 'a');
    expect(getCommand('/key', MODAL_ID)).toBe('/key a');
  });

  it('serializes key args, skipping search and list aux controls (Text / Press a key)', async () => {
    await render(testCtx(), 'Text', 'Press a key', {
      command: '/key',
      args: [{ TYPE: 'input key' }],
      style: { image: 'key.png', image_size: '75%' },
    });
    (document.querySelector('#key-input_7X7') as HTMLInputElement).value = 'a';
    // Aux controls hold decoy values that must never serialize.
    (document.querySelector('#key-list_7X7 .sd-search') as HTMLInputElement).value = 'decoy';
    expect(getCommand('/key', MODAL_ID)).toBe('/key a');
  });

  it('serializes a longtext arg (Text / Write text and press Enter)', async () => {
    await render(testCtx(), 'Text', 'Write text and press Enter', {
      command: '/writeandsend',
      args: [{ TYPE: 'input longtext' }],
      style: { image: 'write.png', image_size: '75%' },
    });
    setValue(fields()[0]!, 'hello');
    expect(getCommand('/writeandsend', MODAL_ID)).toBe('/writeandsend hello');
  });

  it('serializes choice options, skipping hidden panes (Text / Copy)', async () => {
    await render(testCtx(), 'Text', 'Copy', {
      command: '/copy',
      args: [{ TYPE: 'choice', options: [{ TYPE: 'NONE checked' }, { TYPE: 'input text' }] }],
      style: { image: 'copy.png', image_size: '100%' },
    });
    // Default: first (empty) choice selected.
    expect(getCommand('/copy', MODAL_ID)).toBe('/copy ');
    // Switch to the second choice (mirrors showArg_X pane toggling).
    const panes = [...document.querySelectorAll(`div.arg_container[data-arg-modal-id="${MODAL_ID}"]`)] as HTMLElement[];
    panes[0]!.style.display = 'none';
    panes[1]!.style.display = 'block';
    setValue(fields()[2]!, 'hi');
    expect(getCommand('/copy', MODAL_ID)).toBe('/copy hi');
  });

  it('serializes dropdown selection by option id (System / ScreenSaver)', async () => {
    await render(testCtx(), 'System', 'ScreenSaver', {
      command: '/screensaver',
      args: [{ TYPE: 'input dropdown', options: [{ ID: 'NONE' }, { ID: 'full' }, { ID: 'off' }] }],
    });
    (fields()[0]! as HTMLSelectElement).selectedIndex = 1;
    expect(getCommand('/screensaver', MODAL_ID)).toBe('/screensaver full');
  });

  it('serializes multi-item choices positionally (System / Execute script code)', async () => {
    await render(testCtx(), 'System', 'Execute script code', {
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
    });
    // Default: uploaded-file pane (file input contributes nothing).
    expect(getCommand('/exec', MODAL_ID)).toBe('/exec type:uploaded_file');
    const panes = [...document.querySelectorAll(`div.arg_container[data-arg-modal-id="${MODAL_ID}"]`)] as HTMLElement[];
    panes[0]!.style.display = 'none';
    panes[2]!.style.display = 'block';
    setValue(fields()[8]!, 'print(1)');
    expect(getCommand('/exec', MODAL_ID)).toBe('/exec type:single_line<|§|>print(1)');
  });

  it('serializes file upload + number args (Soundboard / Playsound)', async () => {
    await render(testCtx(), 'Soundboard', 'Playsound', {
      command: '/playsound',
      args: [{ TYPE: "input filetype['.mp3']" }, { TYPE: "input number['1','100']", placeholder: '50' }],
      style: { image: 'volume-up.svg', image_size: '70%' },
    });
    setValue(fields()[1]!, '75');
    expect(getCommand('/playsound', MODAL_ID)).toBe('/playsound 75');
  });

  it('serializes usage titles with hidden fragments (Display / CPU)', async () => {
    await render(testCtx(), 'Display', 'CPU', {
      command: "/usage '",
      args: [{ TYPE: 'input usage-title-text', value: 'CPU' }, { TYPE: 'text', value: "' usage_dict['cpu']['usage_percent']" }],
      style: { image: '', image_size: '' },
    });
    expect(getCommand("/usage '", MODAL_ID)).toBe("/usage ' CPU<|§|>' usage_dict['cpu']['usage_percent']");
  });

  it('serializes disk-letter selects with C preselected (Display / Disks)', async () => {
    const ctx = testCtx({ usage_example: { disks: { C: {}, D: {} } } });
    await render(ctx, 'Display', 'Disks', {
      command: "/usage '",
      args: [
        { TYPE: 'input usage-title-text', value: 'Disk' },
        { TYPE: 'text', value: "' usage_dict['disks']['" },
        { TYPE: 'input disk-letter', value: 'Disk' },
        { TYPE: 'text', value: "']['usage_percent']" },
      ],
      style: {},
    });
    // Browsers preselect C via the `selected` attribute; happy-dom parses the
    // attribute without updating selectedness, so select it explicitly.
    (document.querySelector('select')! as HTMLSelectElement).selectedIndex = 0;
    expect(getCommand("/usage '", MODAL_ID)).toBe(
      "/usage ' Disk<|§|>' usage_dict['disks']['<|§|>C<|§|>']['usage_percent']"
    );
  });

  it('serializes gpu selects by device key (Display / GPU)', async () => {
    // The message fragment is a `usage_dict['gpus']` key (`GPU1`, …), never
    // the display name: names miss the lookup (and spaced names break the
    // dotted tile-path eval), leaving the tile at `-`.
    const ctx = testCtx({ usage_example: { gpus: { GPU1: { name: 'RTX 4090' } } } });
    await render(ctx, 'Display', 'GPU', {
      command: "/usage '",
      args: [
        { TYPE: 'input usage-title-text', value: 'GPU' },
        { TYPE: 'text', value: "' usage_dict['gpus']['" },
        { TYPE: 'input available_gpus' },
        { TYPE: 'text', value: "']['usage_percent']" },
      ],
      style: {},
    });
    expect(getCommand("/usage '", MODAL_ID)).toBe(
      "/usage ' GPU<|§|>' usage_dict['gpus']['<|§|>GPU1<|§|>']['usage_percent']"
    );
    // The device name stays visible as the option label.
    expect(document.querySelector('select')!.innerHTML).toContain('>RTX 4090</option>');
  });

  it('serializes folder radios, ignoring the create-folder form (Webdeck / Open a folder)', async () => {
    const ctx = testCtx({ config: { front: { names_color: '', buttons: { folderA: [], folderB: [] } }, settings: {} } });
    await render(ctx, 'Webdeck', 'Open a folder', {
      command: '/folder',
      args: [{ TYPE: 'input webdeck_foldername' }],
    });
    const radios = fields().filter((el) => (el as HTMLInputElement).type === 'radio');
    (radios[1]! as HTMLInputElement).checked = true;
    expect(getCommand('/folder', MODAL_ID)).toBe('/folder folderB');
  });

  it('serializes file-picker text, skipping the picker button (System / Open)', async () => {
    await render(testCtx(), 'System', 'Open', { command: '/start', args: [{ TYPE: 'input file' }] });
    setValue(fields()[0]!, 'C:\\x.exe');
    expect(getCommand('/start', MODAL_ID)).toBe('/start C:\\x.exe');
  });

  it('serializes folder-picker and url args', async () => {
    await render(testCtx(), 'System', 'opendir', { command: '/openfolder', args: [{ TYPE: 'input folderpath' }] });
    setValue(fields()[0]!, 'C:\\games');
    expect(getCommand('/openfolder', MODAL_ID)).toBe('/openfolder C:\\games');

    await render(testCtx(), 'System', 'Open a website', { command: '/start', args: [{ TYPE: 'input url' }] });
    setValue(fields()[0]!, 'https://example.com');
    expect(getCommand('/start', MODAL_ID)).toBe('/start https://example.com');
  });

  const FETCH_ARGS = [
    { TYPE: 'text', value: 'method:' },
    { TYPE: 'input dropdown', options: [{ ID: 'GET', label: 'GET' }, { ID: 'POST', label: 'POST' }] },
    { TYPE: 'text', value: 'url:' },
    { TYPE: 'input url' },
    { TYPE: 'text', value: 'headers:' },
    { TYPE: 'input headers' },
    { TYPE: 'text', value: 'body:' },
    { TYPE: 'input longtext', visibleWhen: { arg: 1, notIn: ['GET', 'HEAD'] } },
    { TYPE: 'text', value: 'timeout:' },
    { TYPE: "input number['1','120']", placeholder: '10' },
  ];

  it('hides the body for GET and skips widget row inputs (Integrations / Fetch URL)', async () => {
    await render(testCtx(), 'Integrations', 'Fetch URL', { command: '/fetch', args: FETCH_ARGS });
    const all = fields();
    const url = all.find((el) => (el as HTMLInputElement).type === 'url')!;
    const timeout = all.find((el) => (el as HTMLInputElement).type === 'number')!;
    setValue(url, 'https://x');
    setValue(timeout, '10');
    // Body hidden for the default GET.
    const bodyPane = document.querySelector('[data-branch="input"][data-arg-id="7"]') as HTMLElement;
    expect(bodyPane.style.display).toBe('none');
    // Widget decoy: row inputs must never serialize directly.
    const decoy = document.querySelector('input[data-nocollect]') as HTMLInputElement;
    decoy.value = 'decoy';
    expect(getCommand('/fetch', MODAL_ID)).toBe(
      '/fetch method:<|§|>GET<|§|>url:<|§|>https://x<|§|>headers:<|§|>body:<|§|>timeout:<|§|>10'
    );
  });

  it('shows the body for POST and joins header rows (Integrations / Fetch URL)', async () => {
    await render(testCtx(), 'Integrations', 'Fetch URL', { command: '/fetch', args: FETCH_ARGS });
    const select = document.querySelector('.args-container select') as HTMLSelectElement;
    select.value = 'POST';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await tick();
    const bodyPane = document.querySelector('[data-branch="input"][data-arg-id="7"]') as HTMLElement;
    expect(bodyPane.style.display).not.toBe('none');
    const names = [...document.querySelectorAll('.kv-row input[placeholder="Name"]')] as HTMLInputElement[];
    const values = [...document.querySelectorAll('.kv-row input[placeholder="Value"]')] as HTMLInputElement[];
    names[0]!.value = 'X-Token';
    names[0]!.dispatchEvent(new Event('input', { bubbles: true }));
    values[0]!.value = 'abc';
    values[0]!.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    const textarea = document.querySelector('.args-container textarea') as HTMLTextAreaElement;
    textarea.value = '{"a":1}';
    expect(getCommand('/fetch', MODAL_ID)).toBe(
      '/fetch method:<|§|>POST<|§|>url:<|§|>headers:<|§|>X-Token: abc<|§|>body:<|§|>{"a":1}<|§|>timeout:'
    );
  });
});
