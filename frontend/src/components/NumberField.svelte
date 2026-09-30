<script lang="ts">
  /** Digit-only number input with a fallback default. */
  interface Props {
    dark: string;
    id: string;
    name: string;
    label?: string | undefined;
    value: string;
    defaultValue?: string | undefined;
    min?: string | undefined;
    required?: boolean | undefined;
    style?: string | undefined;
  }

  let { dark, id, name, label, value, defaultValue, min, required, style }: Props = $props();

  const effective = $derived(value.trim() !== '' ? value.trim() : (defaultValue ?? ''));

  function scrub(event: Event): void {
    const el = event.currentTarget as HTMLInputElement;
    el.value = el.value.replace(/[^0-9]/g, '');
  }
</script>

{#if label !== undefined}<label for={id}> {label} </label>{/if}<input
  required={required}
  class={dark}
  type="number"
  min={min}
  pattern="[0-9]*"
  {style}
  oninput={scrub}
  {id}
  {name}
  value={effective !== '' ? effective : undefined}
/>
