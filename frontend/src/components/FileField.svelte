<script lang="ts">
  import SectionIcon from './SectionIcon.svelte';

  /**
   * Single styled file picker: one button-like drop row showing either the
   * empty hint or the chosen file name. The native input stays in the DOM
   * (same id/accept) so the existing upload wiring keeps working, but it is
   * visually hidden — no doubled "Browse / no file chosen" texts.
   */

  interface Props {
    /** Stable hook id (`image-input_*`, `create-image-bg`, ...). */
    id: string;
    accept: string;
    /** Primary action label, e.g. "Select your file". */
    browseLabel: string;
    /** Shown when nothing is chosen yet. */
    emptyLabel: string;
    /** Optional format hint rendered under the name, e.g. "JPG · PNG". */
    hint?: string | undefined;
  }

  let { id, accept, browseLabel, emptyLabel, hint }: Props = $props();

  let fileName = $state('');

  function onChange(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    fileName = input.files?.[0]?.name ?? '';
  }
</script>

<div class="wd2-dropfile">
  <input type="file" {id} {accept} class="wd2-dropfile-input" onchange={onChange} />
  <label class="wd2-dropfile-label" for={id}>
    <span class="wd2-dropfile-icon" aria-hidden="true">
      <SectionIcon name={fileName !== '' ? 'check' : 'upload'} size={18} />
    </span>
    <span class="wd2-dropfile-text">
      <span class="wd2-dropfile-action">{browseLabel}</span>
      <span class="wd2-dropfile-name" class:chosen={fileName !== ''}>
        {fileName !== '' ? fileName : emptyLabel}
      </span>
      {#if hint !== undefined}
        <span class="wd2-dropfile-hint">{hint}</span>
      {/if}
    </span>
  </label>
</div>
