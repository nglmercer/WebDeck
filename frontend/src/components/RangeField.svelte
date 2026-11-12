<script lang="ts">
  import { id } from '../lib/id';
  let {
    label,
    value,
    min = 0,
    max = 100,
    step = 1,
    unit = '',
    onchange,
  }: {
    label: string;
    value: number;
    min?: number;
    max?: number;
    step?: number;
    unit?: string;
    onchange: (value: number) => void;
  } = $props();
  const inputId = `range-${id()}`;
</script>

<div class="range-field">
  <div class="range-heading">
    <label for={inputId}>{label}</label>
    <div class="number">
      <input
        type="number"
        aria-label={label}
        aria-describedby={unit ? `${inputId}-unit` : undefined}
        {min}
        {max}
        {step}
        {value}
        required
        oninput={(event) => {
          if (event.currentTarget.checkValidity()) onchange(event.currentTarget.valueAsNumber);
        }}
      />{#if unit}<span id={`${inputId}-unit`}>{unit}</span>{/if}
    </div>
  </div>
  <input
    id={inputId}
    type="range"
    {min}
    {max}
    {step}
    {value}
    style={`--fill:${(100 * (value - min)) / (max - min)}%`}
    oninput={(event) => onchange(event.currentTarget.valueAsNumber)}
  />
  <div class="bounds" aria-hidden="true"><span>{min}</span><span>{max}</span></div>
</div>

<style>
  .range-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
  }
  label {
    margin: 0;
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 550;
  }
  .number {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    width: 88px;
    min-height: 36px;
    border: 1px solid var(--border);
    background: var(--canvas);
    border-radius: 6px;
    padding: 0 10px;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .number:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  .number span {
    color: var(--text-muted);
    pointer-events: none;
  }
  .number input {
    min-width: 0;
    width: 100%;
    min-height: 0;
    padding: 7px 0;
    border: 0;
    background: transparent;
    text-align: right;
    appearance: textfield;
    -moz-appearance: textfield;
  }
  .number input:focus-visible {
    outline: none;
  }
  .number input::-webkit-inner-spin-button,
  .number input::-webkit-outer-spin-button {
    appearance: none;
    margin: 0;
  }
  @media (pointer: coarse) {
    .number {
      min-height: 44px;
    }
  }
  input[type='range'] {
    appearance: none;
    width: 100%;
    height: 22px;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 6px;
    border-radius: 8px;
    background: linear-gradient(to right, var(--accent) var(--fill), var(--border) var(--fill));
  }
  input[type='range']::-moz-range-track {
    height: 6px;
    border-radius: 8px;
    background: var(--border);
  }
  input[type='range']::-moz-range-progress {
    height: 6px;
    border-radius: 8px;
    background: var(--accent);
  }
  input[type='range']::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border: 0;
    border-radius: 50%;
    margin-top: -5px;
    background: var(--text);
  }
  input[type='range']::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border: 0;
    border-radius: 50%;
    background: var(--text);
  }
  .bounds {
    display: flex;
    justify-content: space-between;
    color: var(--text-muted);
    font-size: 0.7rem;
  }
</style>
