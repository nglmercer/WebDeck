<script lang="ts">
  /**
   * Vertical step navigation for the config modal: one step per settings
   * category. Selection only changes which category is visible; every
   * section stays mounted so the config form still serializes fully.
   */

  export interface StudioStep {
    id: string;
    label: string;
  }

  interface Props {
    steps: StudioStep[];
    selected: string;
    onSelect: (id: string) => void;
    startIndex?: number | undefined;
  }

  let { steps, selected, onSelect, startIndex = 1 }: Props = $props();
</script>

<nav class="wd2-steps" aria-label="settings sections">
  {#each steps as step, i}
    <button
      type="button"
      class="wd2-step"
      aria-current={step.id === selected ? 'true' : 'false'}
      onclick={() => onSelect(step.id)}
    >
      <span class="n" aria-hidden="true">{startIndex + i}</span>
      <span>{step.label}</span>
    </button>
  {/each}
</nav>
