<script lang="ts">
  import type { Snippet } from 'svelte';

  /**
   * Labeled field wrapper: label + control + optional hint. Pure chrome —
   * the control keeps its own wiring-critical ids/classes.
   */

  interface Props {
    /** Input id the label points at. */
    labelFor?: string | undefined;
    label: string;
    hint?: string | undefined;
    required?: boolean | undefined;
    children?: Snippet | undefined;
  }

  let { labelFor, label, hint, required, children }: Props = $props();
</script>

<div class="wd2-field">
  <label class="wd2-label" for={labelFor}>
    {label}{#if required}<span class="wd2-req" aria-hidden="true"> *</span>{/if}
  </label>
  {#if children !== undefined}{@render children()}{/if}
  {#if hint !== undefined}<p class="wd2-help">{hint}</p>{/if}
</div>
