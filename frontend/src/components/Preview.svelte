<script lang="ts">
  import type { PreviewData } from './preview';
  import SvgSlot from './SvgSlot.svelte';

  /**
   * Live button preview (`.fakeform` content). Style attributes render
   * single-line where the legacy templates spread them over several —
   * CSS-identical, no string test depends on the whitespace.
   */

  interface Props {
    data: PreviewData;
  }

  let { data }: Props = $props();

  /** Broken upload images remove themselves (`onerror="this.remove()"`). */
  function removeImg(event: Event): void {
    (event.currentTarget as HTMLImageElement).remove();
  }
</script>

<!-- svelte-ignore a11y_no_redundant_roles: 1:1 port, upstream carries role="button". -->
<button
  type="button"
  id={data.buttonId ? `button-element_${data.id}` : undefined}
  class="wd_button"
  role="button"
  style={data.buttonStyle}
>
  {#if data.media.kind === 'svg'}
    <SvgSlot slot={data.media.slot} />
  {:else if data.media.src !== null}
    <img
      id="button-image_{data.id}"
      src={data.media.src}
      draggable={false}
      alt={data.media.alt ?? undefined}
      onerror={data.media.removeOnError ? removeImg : undefined}
      style="width: {String(data.media.widthPx)}px; {data.media.fill}"
    />
  {:else}
    <img
      id="button-image_{data.id}"
      draggable={false}
      alt=""
      style="width: {String(data.media.widthPx)}px;"
    />
  {/if}
  {#if data.usageFill !== null}
    <div class="usage">
      <div id="usage-title_{data.id}" class="usage-title" style={data.usageFill}></div>
      <div id="usage-value_{data.id}" class="usage-value" style={data.usageFill}></div>
    </div>
  {/if}
</button>
<p class="buttontext" id="button-text-preview_{data.id}" style={data.textStyle ?? undefined}>
  {data.text}
</p>
