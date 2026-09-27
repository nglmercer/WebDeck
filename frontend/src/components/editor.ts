import { html, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import { colorField, normalizeHexColor } from './fields';

/**
 * Shared `.editorStyle` preview + inputs block, used by both the edit-button
 * modal and the add-button args modal (they rendered the same markup from
 * two copies). Control ids are suffixed with the modal id, exactly as the
 * per-modal wiring (`wireEditModal`, add-button wiring) expects.
 */
export interface EditorStyleOptions {
  dark: string;
  /** Modal id suffix for every control id (`image-input_${id}`, ...). */
  id: string;
  /** Live button preview markup (fakeform content). */
  preview: Html;
  /** Image-size slider/number default, percent without `%`. */
  defaultSize: string;
  /** Configured background color (raw config value, normalized here). */
  backgroundColor: string;
  /** Button-name placeholder. */
  buttonName: string;
  /** Button-name input value, '' when unset. */
  nameValue: string;
}

export function editorStyleBlock(o: EditorStyleOptions): Html {
  const bg = normalizeHexColor(o.backgroundColor);
  const titleInput =
    o.nameValue !== ''
      ? html`<input type="text" id="button-text-input_${o.id}" class="${raw(o.dark)}" placeholder="${o.buttonName}" value="${o.nameValue}" />`
      : html`<input type="text" id="button-text-input_${o.id}" class="${raw(o.dark)}" placeholder="${o.buttonName}" />`;
  return html`<div class="editorStyle">
    <div class="fakeform-container ${raw(o.dark)}">
      <div class="fakeform">${o.preview}</div>
    </div>
    <div class="inputs_container">
      <label for="image-input_${o.id}"> ${text('image')}: </label>
      <input type="file" id="image-input_${o.id}" class="${raw(o.dark)}" />
      <div class="slider-container">
        <label for="image-size-slider_${o.id}"> ${text('image_size')}: </label>
        <input type="range" id="image-size-slider_${o.id}" class="${raw(o.dark)}" name="image-size" min="0" max="100" value="${o.defaultSize}" step="1" />
        <input type="number" id="image-size-value_${o.id}" class="image-size-value ${raw(o.dark)}" min="0" step="1" value="${o.defaultSize}" />
        %
      </div>
      <label for="background-color-input_${o.id}"> ${text('background_color')}: </label>
      ${colorField({
        dark: o.dark,
        containerClass: 'background-color-input-container',
        colorClass: 'background-color-input',
        colorId: `background-color-input_${o.id}`,
        hexClass: '',
        hexId: `background-color-hex_${o.id}`,
        placeholder: text('background_color_hex'),
        value: bg,
      })}
      <label for="button-text-input_${o.id}"> ${text('button_title')}: </label>
      ${titleInput}
    </div>
  </div>`;
}

/** Modal save button (shared by the edit/add-button forms). */
export function editorSaveButton(dark: string, id: string): Html {
  return html`<input type="submit" value="${text('save')}" id="${id}_submit" class="createbutton_submit ${raw(dark)}" style="margin-top: 30px;" />`;
}
