<script lang="ts">
  import { id } from '../lib/id';
  let {
    label,
    value,
    options,
    onchange,
  }: {
    label: string;
    value: string;
    options: { value: string; label: string }[];
    onchange: (value: string) => void;
  } = $props();
  const name = `segment-${id()}`;
</script>

<div class="segments" role="radiogroup" aria-label={label}>
  {#each options as option}
    <label class:selected={value === option.value}>
      <input
        class="choice-input"
        type="radio"
        {name}
        value={option.value}
        checked={value === option.value}
        onchange={() => onchange(option.value)}
      />
      <span>{option.label}</span>
    </label>
  {/each}
</div>

<style>
  .segments {
    display: flex;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    overflow: hidden;
    background: var(--canvas);
  }
  label {
    position: relative;
    flex: 1 1 0;
    min-width: 0;
    margin: 0;
    text-align: center;
    padding: 10px 8px;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .choice-input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    opacity: 0;
    cursor: pointer;
    z-index: 1;
  }
  label.selected {
    background: var(--accent-surface);
    color: var(--text);
  }
  label:hover {
    background: var(--surface-hover);
  }
  label.selected:hover {
    background: var(--accent-surface);
  }
  label:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: -3px;
  }
</style>
