<script lang="ts">
  let {
    label,
    value,
    options,
    onchange,
    hideLabel = false,
  }: {
    label: string;
    value: string;
    options: { value: string; label: string }[];
    onchange: (value: string) => void;
    hideLabel?: boolean;
  } = $props();
</script>

<label class="filter-select">
  <span class:sr-only={hideLabel}>{label}</span>
  <select {value} onchange={(event) => onchange(event.currentTarget.value)}>
    {#each options as option}<option value={option.value}>{option.label}</option>{/each}
  </select>
</label>

<style>
  .filter-select {
    min-width: 0;
    margin: 0;
  }
  select {
    min-width: 0;
    min-height: 44px;
    border-radius: var(--radius-control);
    font-size: 0.8rem;
    color: var(--text);
  }
</style>
