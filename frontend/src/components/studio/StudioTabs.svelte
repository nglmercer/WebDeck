<script lang="ts">
  /**
   * Studio tab strip (v2-estudio): accessible tablist with arrow-key
   * navigation. Pane visibility is owned by the parent (it keeps every
   * pane mounted so form serialization and wiring keep working).
   */

  export interface StudioTab {
    id: string;
    label: string;
  }

  interface Props {
    tabs: StudioTab[];
    selected: string;
    onSelect: (id: string) => void;
    /** Prefix for tab/panel element ids (unique per modal). */
    idPrefix: string;
  }

  let { tabs, selected, onSelect, idPrefix }: Props = $props();

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    event.preventDefault();
    const at = tabs.findIndex((t) => t.id === selected);
    const next =
      event.key === 'ArrowRight'
        ? tabs[(at + 1) % tabs.length]
        : tabs[(at - 1 + tabs.length) % tabs.length];
    if (next) {
      onSelect(next.id);
      document.getElementById(`${idPrefix}-tab-${next.id}`)?.focus();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions: arrow-key delegation for the tablist (focus stays on the tab buttons). -->
<div class="wd2-tabs" role="tablist" tabindex="0" aria-label="modal sections" onkeydown={onKeydown}>
  {#each tabs as tab}
    <button
      type="button"
      role="tab"
      class="wd2-tab"
      id="{idPrefix}-tab-{tab.id}"
      aria-selected={tab.id === selected ? 'true' : 'false'}
      aria-controls="{idPrefix}-pane-{tab.id}"
      tabindex={tab.id === selected ? 0 : -1}
      onclick={() => onSelect(tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</div>
