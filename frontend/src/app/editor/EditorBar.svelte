<script lang="ts">
  import { text } from '../../framework/i18n';

  // Render-once by design: the boot context never changes after mount.
  // No <style> block — global theme CSS cascades into this light DOM.
  // svelte-ignore state_referenced_locally
  const swapHint = text('swap_hint');
  // svelte-ignore state_referenced_locally
  const saveAndExit = text('save_and_exit');
  // svelte-ignore state_referenced_locally
  const quitWithoutSaving = text('quit_without_saving');
  // svelte-ignore state_referenced_locally
  const swapButtons = text('swap_buttons');
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
    <span class="editor-btn-label">{saveAndExit}</span>
    <kbd class="editor-kbd">E</kbd>
  </button>
  <button
    class="button editor-btn"
    id="exitEditorButton"
    title="{quitWithoutSaving} (Q)"
    aria-keyshortcuts="q"
  >
    <img src="static/img/cross.svg" width="16" height="16" class="editor-btn-icon" alt="" />
    <span class="editor-btn-label">{quitWithoutSaving}</span>
    <kbd class="editor-kbd">Q</kbd>
  </button>

  <button
    class="button editor-btn"
    id="swapEditorButton"
    title="{swapButtons} (S)"
    aria-keyshortcuts="s"
  >
    <img src="static/img/swap.png" width="16" height="16" class="editor-btn-icon" alt="" />
    <span class="editor-btn-label" id="swapEditorLabel">{swapButtons}</span>
    <kbd class="editor-kbd">S</kbd>
  </button>
</div>
