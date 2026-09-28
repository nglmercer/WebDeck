<script lang="ts">
  import ColorField from './ColorField.svelte';
  import { normalizeHexColor } from './fields';
  import Preview from './Preview.svelte';
  import type { PreviewData } from './preview';
  import { text } from '../framework/i18n';

  /**
   * Shared `.editorStyle` preview + inputs block, used by both the edit-button
   * modal and the add-button args modal. Control ids are suffixed with the
   * modal id, exactly as the per-modal wiring expects.
   */

  interface Props {
    dark: string;
    /** Modal id suffix for every control id (`image-input_${id}`, ...). */
    id: string;
    /** Live button preview (fakeform content). */
    preview: PreviewData;
    /** Image-size slider/number default, percent without `%`. */
    defaultSize: string;
    /** Configured background color (raw config value, normalized here). */
    backgroundColor: string;
    /** Button-name placeholder. */
    buttonName: string;
    /** Button-name input value, '' when unset. */
    nameValue: string;
  }

  let { dark, id, preview, defaultSize, backgroundColor, buttonName, nameValue }: Props =
    $props();

  const bg = $derived(normalizeHexColor(backgroundColor));
</script>

<div class="editorStyle">
  <div class="fakeform-container {dark}">
    <div class="fakeform"><Preview data={preview} /></div>
  </div>
  <div class="inputs_container">
    <label for="image-input_{id}"> {text('image')}: </label>
    <input type="file" id="image-input_{id}" class={dark} />
    <div class="slider-container">
      <label for="image-size-slider_{id}"> {text('image_size')}: </label>
      <input
        type="range"
        id="image-size-slider_{id}"
        class={dark}
        name="image-size"
        min="0"
        max="100"
        value={defaultSize}
        step="1"
      />
      <input
        type="number"
        id="image-size-value_{id}"
        class="image-size-value {dark}"
        min="0"
        step="1"
        value={defaultSize}
      />
      %
    </div>
    <label for="background-color-input_{id}"> {text('background_color')}: </label>
    <ColorField
      dark={dark}
      containerClass="background-color-input-container"
      colorClass="background-color-input"
      colorId="background-color-input_{id}"
      hexClass=""
      hexId="background-color-hex_{id}"
      placeholder={text('background_color_hex')}
      value={bg}
    />
    <label for="button-text-input_{id}"> {text('button_title')}: </label>
    {#if nameValue !== ''}
      <input
        type="text"
        id="button-text-input_{id}"
        class={dark}
        placeholder={buttonName}
        value={nameValue}
      />
    {:else}
      <input type="text" id="button-text-input_{id}" class={dark} placeholder={buttonName} />
    {/if}
  </div>
</div>
