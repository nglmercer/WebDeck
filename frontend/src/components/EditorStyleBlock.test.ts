import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { initI18n } from '../framework/i18n';
import EditorStyle from './EditorStyle.svelte';
import type { PreviewData } from './preview';

const PREVIEW: PreviewData = {
  id: 'e0X1',
  buttonId: true,
  buttonStyle: 'overflow: hidden;',
  media: { kind: 'img', src: null, alt: '', removeOnError: false, widthPx: 59, fill: '' },
  usageFill: null,
  text: 'preview',
  textStyle: null,
};

initI18n({
  image: 'Image',
  image_size: 'Image size',
  background_color: 'Background color',
  background_color_hex: 'Background color (HEX)',
  button_title: 'Button title',
  save: 'Save',
});

describe('EditorStyle', () => {
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

  function render(props: Record<string, unknown>): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(EditorStyle, { target: host, props }) as unknown as Record<string, never>;
    return host;
  }

  it('renders preview plus suffixed controls', () => {
    const el = render({
      dark: 'dark-theme',
      id: 'e0X1',
      preview: PREVIEW,
      defaultSize: '75',
      backgroundColor: '',
      buttonName: 'My button',
      nameValue: '',
    });
    expect(el.querySelector('.editorStyle')).not.toBeNull();
    expect(el.querySelector('.fakeform .wd_button')).not.toBeNull();
    expect(el.querySelector('#button-image_e0X1')).not.toBeNull();
    expect(el.querySelector('#button-text-preview_e0X1')?.textContent?.trim()).toBe('preview');
    for (const id of [
      'image-input_e0X1',
      'image-size-slider_e0X1',
      'background-color-input_e0X1',
      'background-color-hex_e0X1',
      'button-text-input_e0X1',
    ]) {
      expect(el.querySelector(`#${id}`), id).not.toBeNull();
    }
    expect(el.innerHTML).toContain('Image size');
    // The slider is the single size input — no number twin, no pct badge.
    expect((el.querySelector('#image-size-slider_e0X1') as HTMLInputElement).value).toBe('75');
    expect(el.querySelector('#image-size-value_e0X1')).toBeNull();
    expect(el.querySelector('#size-pct_e0X1')).toBeNull();
  });

  it('normalizes the background color onto both inputs', () => {
    const el = render({
      dark: '',
      id: 'x',
      preview: { ...PREVIEW, id: 'x' },
      defaultSize: '70',
      backgroundColor: 'ff0000',
      buttonName: 'b',
      nameValue: '',
    });
    expect((el.querySelector('#background-color-input_x') as HTMLInputElement).value).toBe('#ff0000');
    expect((el.querySelector('#background-color-hex_x') as HTMLInputElement).value).toBe('#ff0000');
  });

  it('sets the title value only when a name is configured', () => {
    const withName = render({
      dark: '',
      id: 'x',
      preview: { ...PREVIEW, id: 'x' },
      defaultSize: '70',
      backgroundColor: '',
      buttonName: 'b',
      nameValue: 'b',
    });
    const input = withName.querySelector('#button-text-input_x') as HTMLInputElement;
    expect(input.placeholder).toBe('b');
    expect(input.value).toBe('b');
  });
});
