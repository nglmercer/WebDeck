<script lang="ts">
  import ModalShell from '../../components/studio/ModalShell.svelte';
  import { tx } from '../../components/studio/labels';
  import { hide_addbutton_modal } from '../../app/modals';
  import { text } from '../../framework/i18n';
  import type { BootContext } from '../../framework/types';
  import AddBrowser from './AddBrowser.svelte';
  import { addBrowserData } from './browser';

  /** Add-button modal chrome (index.jinja addbutton section). */

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design (see App.svelte).
  // svelte-ignore state_referenced_locally
  const dark = ctx.dark_theme;
  // svelte-ignore state_referenced_locally
  const commandCount = addBrowserData(ctx).categories.reduce(
    (n, c) => n + c.items.reduce((m, i) => m + (i.kind === 'single' ? 1 : i.branch.subs.length), 0),
    0
  );
</script>

<ModalShell
  containerClass="addbutton-modal-container {dark}"
  contentClass="addbutton-modal-content {dark}"
  contentId="addbutton-modal-content"
  headerClass="addbutton-modal-header bold"
  titleClass="addbutton-modal"
  title={text('add_a_button')}
  count={commandCount}
  closeClass="addbutton-modal-close"
  closeIconClass="addbutton-config-modal"
  {dark}
  labelledBy="add-browser-title"
>
  <div class="wd2-browse-bar">
    <div class="wd2-searchwrap">
      <input
        type="text"
        id="addbutton-search"
        class="addbutton-search {dark}"
        placeholder={text('search_commands')}
        autocomplete="off"
        aria-label={text('search_commands')}
      />
    </div>
  </div>
  <div class="addbutton-modal-main">
    <div class="all-commands {dark}">
      <AddBrowser ctx={ctx} />
    </div>
  </div>
  <footer class="wd2-foot">
    <span class="wd2-hint">{tx('studio_browse_hint', 'Pick a command to configure its button')}</span>
    <div class="wd2-actions">
      <button type="button" class="wd2-btn ghost" onclick={() => hide_addbutton_modal()}>
        {tx('cancel', 'Cancel')}
      </button>
    </div>
  </footer>
</ModalShell>
