import { describe, expect, it } from 'vitest';
import { html } from '../framework/html';
import { initI18n } from '../framework/i18n';
import { editorSaveButton, editorStyleBlock } from './editor';

initI18n({
  image: 'Image',
  image_size: 'Image size',
  background_color: 'Background color',
  background_color_hex: 'Background color (HEX)',
  button_title: 'Button title',
  save: 'Save',
});

describe('editorStyleBlock', () => {
  it('renders preview plus suffixed controls', () => {
    const out = editorStyleBlock({
      dark: 'dark-theme',
      id: 'e0X1',
      preview: html`<button>preview</button>`,
      defaultSize: '75',
      backgroundColor: '',
      buttonName: 'My button',
      nameValue: '',
    }).value;
    expect(out).toContain('class="editorStyle"');
    expect(out).toContain('<button>preview</button>');
    expect(out).toContain('id="image-input_e0X1"');
    expect(out).toContain('id="image-size-slider_e0X1"');
    expect(out).toContain('id="image-size-value_e0X1"');
    expect(out).toContain('id="background-color-input_e0X1"');
    expect(out).toContain('id="background-color-hex_e0X1"');
    expect(out).toContain('id="button-text-input_e0X1"');
    expect(out).toContain('Image size');
    // Slider and number input share the default.
    expect(out.match(/value="75"/g) ?? []).toHaveLength(2);
  });

  it('normalizes the background color onto both inputs', () => {
    const out = editorStyleBlock({
      dark: '',
      id: 'x',
      preview: html``,
      defaultSize: '70',
      backgroundColor: 'ff0000',
      buttonName: 'b',
      nameValue: '',
    }).value;
    expect(out.match(/value="#ff0000"/g) ?? []).toHaveLength(2);
  });

  it('sets the title value only when a name is configured', () => {
    const withName = editorStyleBlock({
      dark: '',
      id: 'x',
      preview: html``,
      defaultSize: '70',
      backgroundColor: '',
      buttonName: 'b',
      nameValue: 'b',
    }).value;
    expect(withName).toContain('placeholder="b" value="b"');
    const withoutName = editorStyleBlock({
      dark: '',
      id: 'x',
      preview: html``,
      defaultSize: '70',
      backgroundColor: '',
      buttonName: 'b',
      nameValue: '',
    }).value;
    expect(withoutName).toContain('placeholder="b" />');
  });
});

describe('editorSaveButton', () => {
  it('renders a suffixed submit with a valid margin', () => {
    const out = editorSaveButton('dark-theme', 'e0X1').value;
    expect(out).toContain('type="submit"');
    expect(out).toContain('id="e0X1_submit"');
    expect(out).toContain('value="Save"');
    expect(out).toContain('margin-top: 30px;');
    expect(out).not.toContain('margin-top: 30;');
  });
});
