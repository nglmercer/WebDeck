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
  }: {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    onchange: (value: number) => void;
  } = $props();
  const errorId = `number-${id()}`;
  let error = $state(false);
</script>

<label
  >{label}<input
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
      if (!error) onchange(input.valueAsNumber);
    }}
  /></label
>
{#if error}<p id={errorId} class="field-error">
    {t('ui_number_bounds', { min, max })}
  </p>{/if}

<style>
  .field-error {
    color: var(--error-text);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
