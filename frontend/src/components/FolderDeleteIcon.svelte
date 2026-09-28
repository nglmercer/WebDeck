<script lang="ts">
  import { FOLDER_X_PATH } from './icons';
  import { stringAttrs } from './string-attrs';

  /**
   * Folder-tab delete glyph (bi-x-circle, 16px). The `onclick` handler
   * stays a string attribute (set via `stringAttrs`): browsers compile it
   * and editor code reads sibling `onclick` attributes back. `safeFolder`
   * is the caller-escaped folder id (`"` → `&quot;`).
   */

  interface Props {
    safeFolder: string;
  }

  let { safeFolder }: Props = $props();

  function deleteAttrs(): Record<string, string> {
    return { onclick: `event.stopPropagation(); deleteFolder('${safeFolder}')` };
  }
</script>

<svg
  class="delete-icon"
  use:stringAttrs={deleteAttrs()}
  xmlns="http://www.w3.org/2000/svg"
  width="16"
  height="16"
  fill="currentColor"
  viewBox="0 0 16 16"><path d={FOLDER_X_PATH} /></svg>
