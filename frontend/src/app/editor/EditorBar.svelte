<script lang="ts">
  import { text } from '../../framework/i18n';

  // Render-once by design: the boot context never changes after mount.
  // No <style> block — global theme CSS cascades into this light DOM.
  // Compact buttons: icon + one short word; the full label stays in the
  // hover tooltip (with the shortcut hint) and aria-keyshortcuts.
  // svelte-ignore state_referenced_locally
  const swapHint = text('swap_hint');
  // svelte-ignore state_referenced_locally
  const saveAndExit = text('save_and_exit');
  // svelte-ignore state_referenced_locally
  const saveAndExitShort = text('save_and_exit_short');
  // svelte-ignore state_referenced_locally
  const quitWithoutSaving = text('quit_without_saving');
  // svelte-ignore state_referenced_locally
  const quitWithoutSavingShort = text('quit_without_saving_short');
  // svelte-ignore state_referenced_locally
  const swapButtons = text('swap_buttons');
  // svelte-ignore state_referenced_locally
  const swapButtonsShort = text('swap_buttons_short');
</script>

<div id="EditorButtons" style="display: none;" role="toolbar" aria-label="Editor">
  <span id="swapHint" style="display: none;">{swapHint}</span>
  <button
    class="button editor-btn editor-btn-primary"
    id="SaveExitEditorButton"
    title="{saveAndExit} (E)"
    aria-keyshortcuts="e"
  >
    <img src="static/img/save.svg" width="16" height="16" class="editor-btn-icon" alt="" />
    <span class="editor-btn-label">{saveAndExitShort}</span>
  </button>
  <button
    class="button editor-btn"
    id="exitEditorButton"
    title="{quitWithoutSaving} (Q)"
    aria-keyshortcuts="q"
  >
    <img src="static/img/cross.svg" width="16" height="16" class="editor-btn-icon" alt="" />
    <span class="editor-btn-label">{quitWithoutSavingShort}</span>
  </button>

  <button
    class="button editor-btn"
    id="swapEditorButton"
    title="{swapButtons} (S)"
    aria-keyshortcuts="s"
  >
    <img src="static/img/swap.png" width="16" height="16" class="editor-btn-icon" alt="" />
    <span class="editor-btn-label" id="swapEditorLabel">{swapButtonsShort}</span>
  </button>
</div>
