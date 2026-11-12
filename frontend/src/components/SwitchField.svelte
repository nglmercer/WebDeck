<script lang="ts">
  import { id } from '../lib/id';
  const uid = `switch-${id()}`;
  import Icon from './Icon.svelte';
  let {
    label,
    checked = $bindable(false),
    onchange = () => {},
    disabled = false,
    description = '',
    icon = '',
  }: {
    label: string;
    checked?: boolean;
    onchange?: (value: boolean) => void;
    disabled?: boolean;
    description?: string;
    icon?: string;
  } = $props();
</script>

<label class="switch-field check" class:disabled>
  <span class="caption"
    >{#if icon}<Icon name={icon} size={18} />{/if}<span
      >{label}{#if description}<small id={uid}>{description}</small>{/if}</span
    ></span
  >
  <input
    type="checkbox"
    role="switch"
    aria-label={label}
    aria-describedby={description ? uid : undefined}
    {checked}
    {disabled}
    onchange={(event) => {
      checked = event.currentTarget.checked;
      onchange(checked);
    }}
  />
</label>

<style>
  .switch-field {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 44px;
    margin: 0;
    padding: 4px 0;
    color: var(--text);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .caption {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .caption small {
    display: block;
    color: var(--text-muted);
    font-size: 0.75rem;
    margin-top: 4px;
    line-height: 1.4;
  }
  .disabled {
    opacity: 0.55;
    cursor: default;
  }
  input {
    appearance: none;
    position: relative;
    flex: 0 0 44px;
    width: 44px;
    min-width: 44px;
    height: 44px;
    min-height: 44px;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: inherit;
    border-radius: 8px;
  }
  input::before {
    content: '';
    position: absolute;
    left: 3px;
    top: 11px;
    width: 38px;
    height: 22px;
    border: 1px solid var(--border);
    border-radius: 20px;
    background: var(--canvas);
    box-sizing: border-box;
  }
  input::after {
    content: '';
    position: absolute;
    left: 7px;
    top: 15px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: transform 0.15s;
  }
  input:checked::before {
    background: var(--accent-surface);
    border-color: var(--accent);
  }
  input:checked::after {
    transform: translateX(16px);
    background: var(--text);
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  @media (prefers-reduced-motion: reduce) {
    input::after {
      transition: none;
    }
  }
</style>
