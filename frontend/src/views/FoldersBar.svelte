<script lang="ts">
  import FolderDeleteIcon from '../components/FolderDeleteIcon.svelte';
  import { navigateFolder } from '../features/deck/state.svelte';
  import { text } from '../framework/i18n';
  import type { BootContext } from '../framework/types';
  import { foldersBarData } from './shell';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: the shell mounts one tab bar from a single
  // boot context, so this intentionally captures the initial props.
  // svelte-ignore state_referenced_locally
  const tabs = foldersBarData(ctx);
  // svelte-ignore state_referenced_locally
  const openFolder = text('open_folder');


</script>

<div id="EditorButtons-Folders" class="folders-bar" style="display: none;">
  <details class="folders-dropdown">
    <summary
      class="folders-toggle"
      data-testid="folders-toggle"
      title={openFolder}
      aria-label={openFolder}
    >
      <img
        src="static/img/folder.png"
        width="18"
        height="18"
        class="folders-toggle-icon"
        alt=""
      />
      <img
        src="static/img/chevron_down.svg"
        width="12"
        height="12"
        class="folders-toggle-chevron"
        alt=""
      />
    </summary>
    <div class="folders-panel">
      {#each tabs as tab}
        <div class="folder-tab-wrap">
          <button
            class="button folder-tab EditorButtons-Folder"
            data-folder-target={tab.folderId}
            onclick={() => navigateFolder(tab.folderId)}
            title="{openFolder}: {tab.folderId}"
          >
            <span class="folder-tab-name">{tab.folderId}</span>
            <FolderDeleteIcon folderName={tab.folderId} safeFolder={tab.safe} />
          </button>
        </div>
      {/each}
    </div>
  </details>
</div>
