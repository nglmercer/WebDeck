<script lang="ts">
  import { text } from '../framework/i18n';

  interface Props {
    svgs: string[];
    /**
     * In-app instance: starts hidden (usage polling reveals it on server
     * disconnects — see `app/usage.ts`, which owns the `hidden` class
     * after mount) and the "server disconnected" note fades in after 5s.
     * The boot splash leaves this off (visible immediately, no timer).
     */
    concealed?: boolean;
  }

  let { svgs, concealed = false }: Props = $props();

  // Render-once by design: the boot context never changes after mount.
  // No <style> block — global theme CSS cascades into this light DOM.
  // svelte-ignore state_referenced_locally
  const disconnected = text('server_disconnected');

  let revealed = $state(false);
  $effect(() => {
    if (!concealed) return;
    const timer = setTimeout(() => {
      revealed = true;
    }, 5000);
    return () => clearTimeout(timer);
  });
</script>

<div
  id="loading-screen"
  class:hidden={concealed}
  class:transparent={revealed}
  style:pointer-events={revealed ? 'none' : null}
>
  <div>
    <p id="server-disconnected" class:invisible={!revealed}>{disconnected}...</p>
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
