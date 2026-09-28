<script lang="ts">
  import FolderDeleteIcon from '../components/FolderDeleteIcon.svelte';
  import { stringAttrs } from '../components/string-attrs';
  import { text } from '../framework/i18n';
  import type { BootContext } from '../framework/types';
  import { foldersBarData, type FolderTab } from './shell';

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

  function tabAttrs(tab: FolderTab): Record<string, string> {
    return { onclick: `folder(\`${tab.safe}\`)` };
  }
</script>

<div id="EditorButtons-Folders" class="folders-bar" style="display: none;">
  <span class="folders-bar-label">{openFolder}:</span>
  {#each tabs as tab}
    <div class="folder-tab-wrap">
      <button
        class="button folder-tab EditorButtons-Folder"
        use:stringAttrs={tabAttrs(tab)}
        title="{openFolder}: {tab.folderId}"
      >
        <span class="folder-tab-name">{tab.folderId}</span>
        <FolderDeleteIcon folderName={tab.folderId} safeFolder={tab.safe} />
      </button>
    </div>
  {/each}
</div>
