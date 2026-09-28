<script lang="ts">
  import type { SelectOption } from './fields';

  /** Labeled dropdown. */
  interface Props {
    containerClass: string;
    id: string;
    name: string;
    label?: string | undefined;
    options: SelectOption[];
  }

  let { containerClass, id, name, label, options }: Props = $props();
  let selectEl: HTMLSelectElement | null = $state(null);

  const selectedValue = $derived(options.find((o) => o.selected)?.value);

  $effect(() => {
    // happy-dom doesn't derive selectedness from the `selected` content
    // attribute; syncing .value keeps single-select state 1:1 everywhere
    // (browsers honor both paths identically).
    if (selectEl !== null && selectedValue !== undefined) selectEl.value = selectedValue;
  });
</script>

<div class="setting wd2-field {containerClass}">
  {#if label !== undefined}<label for={id}> {label} </label>{/if}
  <select {id} {name} bind:this={selectEl}>{#each options as opt}<option value={opt.value} selected={opt.selected}>{opt.label}</option>{/each}</select>
</div>
