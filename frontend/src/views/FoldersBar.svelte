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

  function tabAttrs(tab: FolderTab): Record<string, string> {
    return { onclick: `folder(\`${tab.safe}\`)` };
  }
</script>

<div
  id="EditorButtons-Folders"
  style="color: white; display: none; position: fixed; top: 0; right: 0; text-align: right;"
>
  {text('open_folder')}:
  {#each tabs as tab}
    <div style="display: inline-block; margin-right: 10px;">
      <button
        class="button EditorButtons-Folder"
        use:stringAttrs={tabAttrs(tab)}
        style="display: flex; justify-content: center;align-items: center;"
      >
        {tab.folderId}
        <FolderDeleteIcon safeFolder={tab.safe} />
      </button>
    </div>
  {/each}
</div>
