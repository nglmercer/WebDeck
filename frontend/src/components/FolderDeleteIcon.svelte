<script lang="ts">
  import { FOLDER_X_PATH } from './icons';
  import { deleteFolder } from '../app/editor/void';

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

  let { folderName }: Props = $props();

  function remove(event: Event): void {
    event.stopPropagation();
    deleteFolder(folderName);
  }

</script>

<svg
  class="delete-icon folder-tab-delete"
  onclick={remove}
  onkeydown={(event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); remove(event); } }}
  tabindex="0"
  xmlns="http://www.w3.org/2000/svg"
  width="16"
  height="16"
  fill="currentColor"
  viewBox="0 0 16 16"
  role="button"
  aria-label="Delete folder {folderName}"
  ><title>Delete folder {folderName}</title><path d={FOLDER_X_PATH} /></svg>
