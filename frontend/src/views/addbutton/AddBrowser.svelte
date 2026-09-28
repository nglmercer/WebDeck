<script lang="ts">
  import type { BootContext } from '../../framework/types';
  import AddArgsModal from './AddArgsModal.svelte';
  import { addBrowserData, type BrowserLeaf } from './browser';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: the add modal mounts one browser from a single
  // boot context, so this intentionally captures the initial props.
  // svelte-ignore state_referenced_locally
  const data = addBrowserData(ctx);
</script>

{#snippet leafRow(item: BrowserLeaf)}
  {#if item.desc !== ''}
    <div class="addbutton-description"><p>{item.desc}</p></div>
  {/if}
  <button
    arg_modal_ID={item.argModalId}
    class="dropdown-btn no-dropdown {data.dark}"
    id="open-button-{item.argModalId}"
    dropdown-commandTag={item.commandTag}
  >
    {item.title}
  </button>
  <AddArgsModal ctx={ctx} mctx={item.mctx} />
{/snippet}

{#each data.categories as category}
  <button class="dropdown-btn {data.dark}" dropdown-category={category.name}>
    {category.name}
  </button>
  <div class="dropdown-container">
    {#each category.items as item}
      {#if item.kind === 'single'}
        {@render leafRow(item.leaf)}
      {:else}
        {#if item.branch.desc !== ''}
          <div class="addbutton-description"><p>{item.branch.desc}</p></div>
        {/if}
        <button class="dropdown-btn {data.dark}" dropdown-category={item.branch.command}>
          {item.branch.title}
        </button>
        <div class="dropdown-container">
          <div class="dropdown-item-container">
            {#each item.branch.subs as sub}
              {@render leafRow(sub)}
            {/each}
          </div>
        </div>
      {/if}
    {/each}
  </div>
{/each}
