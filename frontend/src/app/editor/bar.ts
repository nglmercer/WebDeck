// Editor toolbar view (extracted from editor.ts).

import { html, type Html } from '../../framework/html';
import { text } from '../../framework/i18n';

export function editorBarView(): Html {
  return html`
    <div id="EditorButtons" style="display: none;">
      <span id="swapHint" style="display: none;">${text('swap_hint')}</span>
      <button class="button" id="SaveExitEditorButton">
        <img src="static/img/save.svg" width="20" height="20" id="EditorButtonLogo" />
        [E] ${text('save_and_exit')}
      </button>
      <button class="button" id="exitEditorButton"> [Q] ${text('quit_without_saving')} </button>

      <button class="button" id="swapEditorButton">
        <img src="static/img/swap.png" width="20" height="20" id="EditorButtonLogo" />
        [S] ${text('swap_buttons')}
      </button>
    </div>
  `;
}
