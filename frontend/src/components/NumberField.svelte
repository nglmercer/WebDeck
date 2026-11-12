<script lang="ts">
  import { useTranslations } from '../lib/i18n';
  const t = useTranslations();

  import { id } from '../lib/id';
  let {
    label,
    value,
    min,
    max,
    step = 1,
    onchange,
    controls = false,
    unit = '',
    onvalidity = () => {},
  }: {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    onchange: (value: number) => void;
    controls?: boolean;
    unit?: string;
    onvalidity?: (valid: boolean) => void;
  } = $props();
  const errorId = `number-${id()}`;
  let error = $state(false);
</script>

<div class="number-field">
  <label for={errorId + '-input'}
    >{label}{#if unit}<span class="unit">{unit}</span>{/if}</label
  >
  <div class="number-input" class:with-controls={controls}>
    {#if controls}<button
        type="button"
        aria-label={t('ui_decrease_value', { label })}
        disabled={value <= min}
        onclick={() => {
          error = false;
          onvalidity(true);
          onchange(Math.max(min, value - step));
        }}>−</button
      >{/if}
    <input
      id={errorId + '-input'}
      type="number"
      {value}
      {min}
      {max}
      {step}
      required
      aria-invalid={error}
      aria-describedby={error ? errorId : undefined}
      oninput={(event) => {
        const input = event.currentTarget;
        error = !Number.isFinite(input.valueAsNumber) || !input.checkValidity();
        onvalidity(!error);
        if (!error) onchange(input.valueAsNumber);
      }}
    />
    {#if controls}<button
        type="button"
        aria-label={t('ui_increase_value', { label })}
        disabled={value >= max}
        onclick={() => {
          error = false;
          onvalidity(true);
          onchange(Math.min(max, value + step));
        }}>+</button
      >{/if}
  </div>
</div>
{#if error}<p id={errorId} class="field-error">
    {t('ui_number_bounds', { min, max })}
  </p>{/if}

<style>
  .number-field {
    min-width: 0;
  }
  .number-field label {
    flex-direction: row;
    align-items: baseline;
    gap: 4px;
  }
  .unit {
    margin-left: 4px;
    color: var(--text-muted);
  }
  .with-controls {
    display: flex;
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    overflow: hidden;
    background: var(--canvas);
  }
  .with-controls input {
    text-align: center;
    min-width: 0;
    border: 0;
    border-radius: 0;
    padding: 8px 2px;
    appearance: textfield;
    -moz-appearance: textfield;
  }
  .with-controls input::-webkit-inner-spin-button,
  .with-controls input::-webkit-outer-spin-button {
    appearance: none;
    margin: 0;
  }
  .with-controls button {
    flex: 0 0 36px;
    padding: 8px;
    min-height: 44px;
    min-width: 36px;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .with-controls button:hover {
    background: var(--surface-hover);
  }
  .with-controls:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .with-controls input:focus-visible,
  .with-controls button:focus-visible {
    outline-offset: -3px;
  }

  .field-error {
    color: var(--error-text);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
