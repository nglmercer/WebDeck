<script lang="ts">
  import { onMount } from 'svelte';
  import { wireModals, resetModalState } from '../app/modals';
  import { isSwapMode, toggleEditorMode } from '../app/editor';
  import { cancelUploads } from '../api/uploads';
  import { stopUsageLoop } from '../app/usage';
  import { socketHolder } from '../app/state';
  import EditorBar from '../app/editor/EditorBar.svelte';
  import type { BootContext } from '../framework/types';
  import AddModal from './addbutton/AddModal.svelte';
  import Grid from './Grid.svelte';
  import Config from './Config.svelte';
  import LoadingScreen from './LoadingScreen.svelte';
  import Shell from './Shell.svelte';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: renderApp mounts a single boot context. No
  // <style> block on purpose — the global theme stylesheets must keep
  // cascading into this light DOM, exactly as with the previous
  // innerHTML render.
  onMount(() => {
    const cleanup = wireModals(() => toggleEditorMode(), isSwapMode);
    return () => { cleanup(); resetModalState(); stopUsageLoop(); cancelUploads(); socketHolder.socket?.disconnect(); socketHolder.socket = null; };
  });
</script>

<LoadingScreen svgs={ctx.svgs} concealed />
<Shell ctx={ctx} />
<div id="deck-scale"><Grid ctx={ctx} /></div>
<EditorBar />
<Config ctx={ctx} />
<AddModal ctx={ctx} />
