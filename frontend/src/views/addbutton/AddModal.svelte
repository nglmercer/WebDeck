<script lang="ts">
  import CloseIcon from '../../components/CloseIcon.svelte';
  import { text } from '../../framework/i18n';
  import type { BootContext } from '../../framework/types';
  import AddBrowser from './AddBrowser.svelte';

  /** Add-button modal chrome (index.jinja addbutton section). */

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design (see App.svelte).
  // svelte-ignore state_referenced_locally
  const dark = ctx.dark_theme;
</script>

<div class="addbutton-modal-container {dark}">
  <div class="addbutton-modal-content {dark}" id="addbutton-modal-content">
    <div class="addbutton-modal-header bold">
      <h1 class="addbutton-modal"> {text('add_a_button')} </h1>
      <div class="addbutton-modal-close">
        <CloseIcon cls="addbutton-config-modal" dark={dark} />
      </div>
    </div>
    <div class="addbutton-modal-main">
      <br class="addbutton-container {dark}" />
      <input
        type="text"
        id="addbutton-search"
        class="addbutton-search {dark}"
        placeholder={text('search_commands')}
      />
      <div class="all-commands {dark}">
        <AddBrowser ctx={ctx} />
      </div>
    </div>
  </div>
</div>
