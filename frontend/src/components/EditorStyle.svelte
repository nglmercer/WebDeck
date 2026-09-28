<script lang="ts">
  import ColorField from './ColorField.svelte';
  import { normalizeHexColor } from './fields';
  import FileField from './FileField.svelte';
  import Preview from './Preview.svelte';
  import type { PreviewData } from './preview';
  import StudioField from './studio/StudioField.svelte';
  import { text } from '../framework/i18n';

  /**
   * Shared button-look block, used by both the edit-button modal and the
   * add-button args modal. Control ids are suffixed with the modal id,
   * exactly as the per-modal wiring expects.
   *
   * Studio split layout: `mode="preview"` renders only the live tile (for
   * the modal sidebar) and `mode="controls"` only the appearance inputs
   * (for the Appearance tab). The default `full` keeps the legacy combined
   * block. Preview ids and control ids never overlap, so both modes can
   * mount side by side for the same modal id.
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
    mode?: 'full' | 'preview' | 'controls' | undefined;
  }

  let {
    dark,
    id,
    preview,
    defaultSize,
    backgroundColor,
    buttonName,
    nameValue,
    mode = 'full',
  }: Props = $props();

  const bg = $derived(normalizeHexColor(backgroundColor));
</script>

{#snippet previewBlock()}
  <div class="fakeform-container {dark}">
    <div class="fakeform"><Preview data={preview} /></div>
  </div>
{/snippet}

{#snippet controlsBlock()}
  <div class="inputs_container">
    <StudioField label={text('image')} labelFor="image-input_{id}">
      <FileField
        id="image-input_{id}"
        accept="image/*"
        browseLabel={text('select_your_file')}
        emptyLabel={text('no_file_chosen')}
      />
    </StudioField>
    <StudioField label={text('image_size')} labelFor="image-size-slider_{id}">
      <div class="slider-container wd2-size-single">
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
      </div>
    </StudioField>
    <StudioField label={text('background_color')} labelFor="background-color-hex_{id}">
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
    </StudioField>
    <StudioField label={text('button_title')} labelFor="button-text-input_{id}">
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
    </StudioField>
  </div>
{/snippet}

<div class="editorStyle" data-mode={mode}>
  {#if mode !== 'controls'}
    {@render previewBlock()}
  {/if}
  {#if mode !== 'preview'}
    {@render controlsBlock()}
  {/if}
</div>
