<script lang="ts" generics="T extends string | number">
  import { id } from '../../../lib/id';
  let {
    label,
    value,
    options,
    onchange,
    hideLabel = false,
  }: {
    label: string;
    value: T;
    options: readonly { value: T; label: string; description?: string }[];
    onchange: (value: T) => void;
    hideLabel?: boolean;
  } = $props();
  const name = `choice-${id()}`;
</script>

<fieldset class="choice-field">
  <legend class:sr-only={hideLabel}>{label}</legend>
  <div class="choices">
    {#each options as option}<label class:chosen={value === option.value}>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={value === option.value}
          onchange={() => onchange(option.value)}
        />
        <span
          ><strong>{option.label}</strong>{#if option.description}<small>{option.description}</small
            >{/if}</span
        >
      </label>{/each}
  </div>
</fieldset>

<style>
  .choice-field {
    border: 0;
    padding: 0;
    margin: 16px 0;
    min-width: 0;
  }
  legend {
    margin-bottom: 8px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  label {
    cursor: pointer;
    display: flex;
    flex-direction: row;
    align-items: center;
    flex: 1 1 110px;
    gap: 8px;
    margin: 0;
    min-height: 44px;
    padding: 10px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    background: var(--canvas);
    color: var(--text-muted);
  }
  label:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  label:hover {
    border-color: var(--border);
  }
  label.chosen {
    border-color: var(--text-muted);
    background: var(--surface-raised);
    color: var(--text);
  }
  input {
    appearance: none;
    flex: 0 0 14px;
    width: 14px;
    height: 14px;
    min-height: 0;
    padding: 0;
    margin: 0;
    border-radius: 50%;
    border: 1px solid var(--text-muted);
    background: transparent;
  }
  input:checked {
    background: var(--text);
    border: 4px solid var(--surface-raised);
    outline: 1px solid var(--text-muted);
  }
  strong {
    font-size: 0.8rem;
    font-weight: 500;
  }
  small {
    display: block;
    font-size: 0.75rem;
    margin-top: 4px;
    line-height: 1.5;
  }
</style>
