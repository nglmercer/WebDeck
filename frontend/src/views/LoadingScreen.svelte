<script lang="ts">
  import { text } from '../framework/i18n';

  interface Props {
    svgs: string[];
  }

  let { svgs }: Props = $props();

  // Render-once by design: the boot context never changes after mount.
  // No <style> block — global theme CSS cascades into this light DOM.
  // svelte-ignore state_referenced_locally
  const disconnected = text('server_disconnected');
</script>

<div id="loading-screen">
  <div>
    <p id="server-disconnected" class="invisible">{disconnected}...</p>
    <div class="loadingspinner">
      <div id="square1"></div>
      <div id="square2"></div>
      <div id="square3"></div>
      <div id="square4"></div>
      <div id="square5"></div>
    </div>
  </div>
  <div class="invisible">
    {#each svgs as svg}
      <!-- svelte-ignore a11y_missing_attribute: 1:1 port, preload pixels in a hidden div. -->
      <img src={svg} />
    {/each}
  </div>
</div>
