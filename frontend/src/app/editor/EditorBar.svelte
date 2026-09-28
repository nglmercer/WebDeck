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

<div id="EditorButtons" style="display: none;">
  <span id="swapHint" style="display: none;">{swapHint}</span>
  <button class="button" id="SaveExitEditorButton">
    <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative button glyph. -->
    <img src="static/img/save.svg" width="20" height="20" id="EditorButtonLogo" />
    [E] {saveAndExit}
  </button>
  <button class="button" id="exitEditorButton"> [Q] {quitWithoutSaving} </button>

  <button class="button" id="swapEditorButton">
    <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative button glyph. -->
    <img src="static/img/swap.png" width="20" height="20" id="EditorButtonLogo" />
    [S] {swapButtons}
  </button>
</div>
