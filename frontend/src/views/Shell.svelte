<script lang="ts">
  import { submitDeck } from '../features/deck/submit';
  import { asString, get } from '../framework/types';
  import type { BootContext } from '../framework/types';
  import FoldersBar from './FoldersBar.svelte';
  import { backgroundVideoSrc, shellCss, showConsoleForm } from './shell';

  interface Props {
    ctx: BootContext;
  }

  interface ShellData {
    videoSrc: string;
    showConsole: boolean;
    dark: string;
    styleTag: string;
  }

  function renderShell(ctx: BootContext): ShellData {
    return {
      videoSrc: backgroundVideoSrc(ctx.random_bg),
      showConsole: showConsoleForm(ctx),
      dark: ctx.dark_theme,
      // Svelte reserves <style> in templates, so the dynamic per-boot CSS
      // stays a string hole (placement-preserving: exactly where the
      // legacy template rendered it).
      styleTag: `<style>${shellCss(ctx)}</style>`,
    };
  }

  let { ctx }: Props = $props();

  // Render-once by design: renderApp mounts a single boot context, so this
  // intentionally captures the initial prop values. No <style> block on
  // purpose — the global theme stylesheets must keep cascading into this
  // light DOM, exactly as with the previous innerHTML render.
  // svelte-ignore state_referenced_locally
  const shell = renderShell(ctx);
</script>

{#if shell.videoSrc !== ''}
  <div class="background-video">
    <!-- svelte-ignore a11y_media_has_caption: 1:1 port, decorative ambient background. -->
    <video autoplay muted loop class="background-video">
      <source src={shell.videoSrc} type="video/mp4" />
      Your browser does not support the video tag...
    </video>
  </div>
{/if}
{#if shell.showConsole}
  <form class="form" onsubmit={(event) => submitDeck(event, asString(get(ctx.config, 'settings', 'data_transfer_method')))}>
    <!-- svelte-ignore a11y_label_has_associated_control: 1:1 port of the debug console label. -->
    <label style="color: white;">Console:</label><br />
    <input type="text" class={'message ' + shell.dark} /><br />
    <button type="submit">Submit</button>
  </form>
{/if}
{@html shell.styleTag}
<FoldersBar ctx={ctx} />
