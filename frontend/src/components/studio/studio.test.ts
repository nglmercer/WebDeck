import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EditorStyle from '../EditorStyle.svelte';
import { initI18n } from '../../framework/i18n';
import type { PreviewData } from '../preview';
import { refreshStudioPreview } from '../../views/studio-preview';
import { wireModalA11y } from './a11y';
import ModalShell from './ModalShell.svelte';
import { tx } from './labels';
import StudioField from './StudioField.svelte';
import StudioSteps from './StudioSteps.svelte';
import StudioTabs from './StudioTabs.svelte';

const PREVIEW: PreviewData = {
  id: 'e0X1',
  buttonId: true,
  buttonStyle: 'overflow: hidden;',
  media: { kind: 'img', src: null, alt: '', removeOnError: false, widthPx: 59, fill: '' },
  usageFill: null,
  text: 'preview',
  textStyle: null,
};

function editorProps(mode: 'full' | 'preview' | 'controls'): Record<string, unknown> {
  return {
    dark: 'dark-theme',
    id: 'e0X1',
    preview: PREVIEW,
    defaultSize: '75',
    backgroundColor: '',
    buttonName: 'My button',
    nameValue: '',
    mode,
  };
}

describe('studio components', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  function render(component: unknown, props: Record<string, unknown>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(component as never, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('tabs mark selection and report clicks', async () => {
    initI18n({});
    const onSelect = vi.fn();
    const el = render(StudioTabs, {
      tabs: [
        { id: 'args', label: 'Parameters' },
        { id: 'look', label: 'Appearance' },
      ],
      selected: 'args',
      onSelect,
      idPrefix: 'edit-e0X0',
    });
    await tick();
    const tabs = el.querySelectorAll('[role="tab"]');
    expect(tabs).toHaveLength(2);
    expect(tabs[0]?.getAttribute('aria-selected')).toBe('true');
    expect(tabs[1]?.getAttribute('aria-selected')).toBe('false');
    (tabs[1] as HTMLButtonElement).click();
    expect(onSelect).toHaveBeenCalledWith('look');
  });

  it('tabs move with arrow keys', async () => {
    initI18n({});
    const onSelect = vi.fn();
    const el = render(StudioTabs, {
      tabs: [
        { id: 'a', label: 'A' },
        { id: 'b', label: 'B' },
      ],
      selected: 'a',
      onSelect,
      idPrefix: 't',
    });
    await tick();
    el.querySelector('[role="tablist"]')?.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })
    );
    expect(onSelect).toHaveBeenCalledWith('b');
  });

  it('steps number entries and mark the current one', async () => {
    initI18n({});
    const onSelect = vi.fn();
    const el = render(StudioSteps, {
      steps: [
        { id: 'settings', label: 'Settings' },
        { id: 'visuals', label: 'Visuals' },
      ],
      selected: 'visuals',
      onSelect,
    });
    await tick();
    const steps = el.querySelectorAll('.wd2-step');
    expect(steps).toHaveLength(2);
    expect(steps[0]?.querySelector('.n')?.textContent).toBe('1');
    expect(steps[1]?.querySelector('.n')?.textContent).toBe('2');
    expect(steps[1]?.getAttribute('aria-current')).toBe('true');
    (steps[0] as HTMLButtonElement).click();
    expect(onSelect).toHaveBeenCalledWith('settings');
  });

  it('field wraps a label, hint, and control', async () => {
    initI18n({});
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(StudioField, {
      target: host,
      props: { labelFor: 'x', label: 'Name', hint: 'Shown on the tile', required: true },
    }) as unknown as Record<string, never>;
    await tick();
    expect(host.querySelector('label.wd2-label')?.textContent).toContain('Name');
    expect(host.querySelector('.wd2-req')).not.toBeNull();
    expect(host.querySelector('.wd2-help')?.textContent).toBe('Shown on the tile');
  });

  it('editor preview mode renders only the tile', () => {
    initI18n({
      image: 'Image',
      image_size: 'Image size',
      background_color: 'Background color',
      background_color_hex: 'Background color (HEX)',
      button_title: 'Button title',
      select_your_file: 'Browse',
      no_file_chosen: 'No file',
    });
    const el = render(EditorStyle, editorProps('preview'));
    expect(el.querySelector('.editorStyle')).not.toBeNull();
    expect(el.querySelector('.fakeform .wd_button')).not.toBeNull();
    expect(el.querySelector('#image-input_e0X1')).toBeNull();
    expect(el.querySelector('#button-text-input_e0X1')).toBeNull();
  });

  it('editor controls mode renders only the inputs', () => {
    initI18n({
      image: 'Image',
      image_size: 'Image size',
      background_color: 'Background color',
      background_color_hex: 'Background color (HEX)',
      button_title: 'Button title',
      select_your_file: 'Browse',
      no_file_chosen: 'No file',
    });
    const el = render(EditorStyle, editorProps('controls'));
    expect(el.querySelector('.fakeform')).toBeNull();
    for (const id of [
      'image-input_e0X1',
      'image-size-slider_e0X1',
      'background-color-input_e0X1',
      'background-color-hex_e0X1',
      'button-text-input_e0X1',
    ]) {
      expect(el.querySelector(`#${id}`), id).not.toBeNull();
    }
    // Single size control: the value lives in the preview, not beside it.
    expect(el.querySelector('#image-size-value_e0X1')).toBeNull();
    expect(el.querySelector('#size-pct_e0X1')).toBeNull();
    // One file row: action label + name, native input visually hidden.
    expect(el.querySelector('.wd2-dropfile-label')).not.toBeNull();
    expect(el.querySelector('#image-input_e0X1')?.className).toContain('wd2-dropfile-input');
  });
});

describe('tx', () => {
  it('falls back to English for missing keys', () => {
    initI18n({});
    expect(tx('studio_tab_params', 'Parameters')).toBe('Parameters');
  });

  it('prefers the translated string when present', () => {
    initI18n({ studio_tab_params: 'Parámetros' });
    expect(tx('studio_tab_params', 'Parameters')).toBe('Parámetros');
  });
});

describe('refreshStudioPreview', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('mirrors size, color, and key into the sidebar readouts', () => {
    document.body.innerHTML = `
      <div id="edit-modal-container-e0X0">
        <input id="image-size-slider_e0X0" value="80">
        <input id="background-color-hex_e0X0" value="#112233">
        <input id="key-input_e0X0" value="enter">
        <b id="meta-size_e0X0"></b><b id="meta-color_e0X0"></b><b id="meta-key_e0X0"></b>
      </div>`;
    refreshStudioPreview('e0X0');
    expect(document.querySelector('#meta-size_e0X0')?.textContent).toBe('80 %');
    expect(document.querySelector('#meta-color_e0X0')?.textContent).toBe('#112233');
    expect(document.querySelector('#meta-key_e0X0')?.textContent).toBe('enter');
  });

  it('shows dashes for missing values and never throws without a modal', () => {
    document.body.innerHTML = `<div id="edit-modal-container-e9X9"></div>`;
    expect(() => refreshStudioPreview('e9X9')).not.toThrow();
    expect(() => refreshStudioPreview('missing')).not.toThrow();
  });
});

describe('ModalShell', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  it('renders hook classes, dialog semantics, and title badge', async () => {
    initI18n({});
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(ModalShell, {
      target: host,
      props: {
        containerClass: 'editbutton-modal-container dark-theme',
        containerId: 'edit-modal-container-e0X0',
        containerAttrs: { 'data-edit-modal-id': 'e0X0' },
        contentClass: 'editbutton-modal-content dark-theme',
        headerClass: 'editbutton-modal-header bold',
        titleClass: 'editbutton-modal',
        title: 'Configura tu botón',
        badge: '/key',
        closeClass: 'editbutton-modal-close',
        closeIconClass: 'editbutton-config-modal',
        dark: 'dark-theme',
        labelledBy: 'edit-e0X0-title',
      },
    }) as unknown as Record<string, never>;
    await tick();
    const container = host.querySelector('#edit-modal-container-e0X0') as HTMLElement;
    expect(container.className).toContain('editbutton-modal-container');
    // Parsed attribute form is lowercase (matches modals.ts lookups).
    expect(container.getAttribute('data-edit-modal-id')).toBe('e0X0');
    const dialog = host.querySelector('[data-wd2-dialog]') as HTMLElement;
    expect(dialog.getAttribute('role')).toBe('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect(dialog.getAttribute('aria-labelledby')).toBe('edit-e0X0-title');
    expect(dialog.getAttribute('tabindex')).toBe('-1');
    expect(host.querySelector('h1.editbutton-modal')?.getAttribute('id')).toBe('edit-e0X0-title');
    expect(host.querySelector('.wd2-cmd')?.textContent).toBe('/key');
    expect(host.querySelector('.editbutton-modal-close svg')).not.toBeNull();
  });
});

describe('wireModalA11y', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  async function flush(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, 20));
  }

  it('focuses opened dialogs and restores focus on close', async () => {
    document.body.innerHTML = `
      <button id="opener">open</button>
      <div class="modal-container" id="mc">
        <div class="modal-content" role="dialog" data-wd2-dialog="t" tabindex="-1" id="dlg">
          <h1 id="t">Title</h1>
        </div>
      </div>`;
    wireModalA11y();
    (document.querySelector('#opener') as HTMLElement).focus();
    (document.querySelector('#mc') as HTMLElement).style.display = 'block';
    await flush();
    expect((document.activeElement as HTMLElement | null)?.id).toBe('dlg');
    (document.querySelector('#mc') as HTMLElement).style.display = 'none';
    await flush();
    expect((document.activeElement as HTMLElement | null)?.id).toBe('opener');
  });

  it('wires each container only once', async () => {
    document.body.innerHTML = `
      <div class="modal-container" id="mc">
        <div class="modal-content" role="dialog" data-wd2-dialog="t" tabindex="-1" id="dlg"></div>
      </div>`;
    wireModalA11y();
    wireModalA11y();
    (document.querySelector('#mc') as HTMLElement).style.display = 'block';
    await flush();
    expect((document.activeElement as HTMLElement | null)?.id).toBe('dlg');
  });
});
