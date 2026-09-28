<script lang="ts">
  import { FOLDER_X_PATH } from './icons';
  import { stringAttrs } from './string-attrs';

  /**
   * Folder-tab delete glyph (bi-x-circle, 16px). The `onclick` handler
   * stays a string attribute (set via `stringAttrs`): browsers compile it
   * and editor code reads sibling `onclick` attributes back. `safeFolder`
   * is the caller-escaped folder id (`"` → `&quot;`); `folderName` is the
   * raw id for the hover tooltip and accessible name.
   */

  interface Props {
    folderName: string;
    safeFolder: string;
  }

  let { folderName, safeFolder }: Props = $props();

  function deleteAttrs(): Record<string, string> {
    return { onclick: `event.stopPropagation(); deleteFolder('${safeFolder}')` };
  }
</script>

<svg
  class="delete-icon folder-tab-delete"
  use:stringAttrs={deleteAttrs()}
  xmlns="http://www.w3.org/2000/svg"
  width="16"
  height="16"
  fill="currentColor"
  viewBox="0 0 16 16"
  role="img"
  aria-label="Delete folder {folderName}"
  ><title>Delete folder {folderName}</title><path d={FOLDER_X_PATH} /></svg>
