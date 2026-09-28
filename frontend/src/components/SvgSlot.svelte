<script lang="ts">
  /**
   * Placeholder span for an async-inlined SVG (resolved by `hydrateSvgs`).
   * The slot registration happens in the data layer (`svgSlotId`); this
   * only renders the marker the hydrator queries.
   */

  interface Props {
    slot: number;
  }

  let { slot }: Props = $props();
</script>

<span data-svg-slot={String(slot)} style="display:contents"></span>
