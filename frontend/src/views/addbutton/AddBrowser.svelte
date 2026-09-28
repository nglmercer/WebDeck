<script lang="ts">
  import SectionIcon from '../../components/SectionIcon.svelte';
  import SvgSlot from '../../components/SvgSlot.svelte';
  import type { BootContext } from '../../framework/types';
  import AddArgsModal from './AddArgsModal.svelte';
  import { addBrowserData, type BrowserLeaf, type BrowserRowIcon } from './browser';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: the add modal mounts one browser from a single
  // boot context, so this intentionally captures the initial props.
  // svelte-ignore state_referenced_locally
  const data = addBrowserData(ctx);
</script>

{#snippet rowIcon(icon: BrowserRowIcon)}
  {#if icon.kind === 'glyph'}
    <SectionIcon name={icon.name} size={18} />
  {:else}
    <!-- Art renders in a light well (deck-tile preview); the adjacent
         title carries the accessible name, so the image is decorative. -->
    <span class="wd2-rowicon">
      {#if icon.kind === 'svg'}
        <SvgSlot slot={icon.slot} />
      {:else}
        <img src={icon.src} draggable={false} alt="" style={icon.fill} />
      {/if}
    </span>
  {/if}
{/snippet}

{#snippet leafRow(item: BrowserLeaf)}
  <!-- .wd2-leaf wraps desc + opener; the description stays the button's
       previous sibling so the live filter keeps pairing them. -->
  <div class="wd2-leaf">
    {#if item.desc !== ''}
      <div class="addbutton-description"><p>{item.desc}</p></div>
    {/if}
    <button
      arg_modal_ID={item.argModalId}
      class="dropdown-btn no-dropdown {data.dark}"
      id="open-button-{item.argModalId}"
      data-testid="add-leaf"
      dropdown-commandTag={item.commandTag}
    >
      {@render rowIcon(item.icon)}
      {item.title}
    </button>
    <AddArgsModal ctx={ctx} mctx={item.mctx} />
  </div>
{/snippet}

{#each data.categories as category}
  <div class="wd2-cat-card">
    <button class="dropdown-btn {data.dark}" dropdown-category={category.name}>
      <SectionIcon name={category.icon} size={18} />
      {category.name}
    </button>
    <div class="dropdown-container">
      {#each category.items as item}
        {#if item.kind === 'single'}
          {@render leafRow(item.leaf)}
        {:else}
          <div class="wd2-branch">
            {#if item.branch.desc !== ''}
              <div class="addbutton-description"><p>{item.branch.desc}</p></div>
            {/if}
            <button class="dropdown-btn {data.dark}" dropdown-category={item.branch.command}>
              {@render rowIcon(item.branch.icon)}
              {item.branch.title}
            </button>
            <div class="dropdown-container">
              <div class="dropdown-item-container">
                {#each item.branch.subs as sub}
                  {@render leafRow(sub)}
                {/each}
              </div>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  </div>
{/each}
