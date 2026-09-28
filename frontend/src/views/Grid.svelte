<script lang="ts">
  import { stringAttrs } from '../components/string-attrs';
  import DeleteXIcon from '../components/DeleteXIcon.svelte';
  import PencilIcon from '../components/PencilIcon.svelte';
  import PlusIcon from '../components/PlusIcon.svelte';
  import SvgSlot from '../components/SvgSlot.svelte';
  import { text } from '../framework/i18n';
  import type { BootContext } from '../framework/types';
  import EditModal from './EditModal.svelte';
  import { gridData, type CellData } from './grid';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: renderApp mounts a single boot context, so this
  // intentionally captures the initial prop values. No <style> block on
  // purpose — the global theme stylesheets must keep cascading into this
  // light DOM, exactly as with the previous innerHTML render.
  // svelte-ignore state_referenced_locally
  const grid = gridData(ctx);
  // svelte-ignore state_referenced_locally
  const addLabel = text('add_a_button');

  /** Raw handler attrs for /folder buttons (empty otherwise). */
  function folderAttrs(cell: CellData): Record<string, string> {
    if (cell.kind !== 'button' || cell.folderHandler === null) return {};
    return { onclick: cell.folderHandler, onclickhandler: cell.folderHandler };
  }

  /**
   * Hover tooltip + accessible name for a tile (undefined omits the
   * attribute). Falls back to the command when the tile is unnamed.
   */
  function tileTitle(cell: CellData): string | undefined {
    if (cell.kind !== 'button') return undefined;
    if (cell.name !== '') return cell.name;
    return cell.message !== '' ? cell.message : undefined;
  }

  /** Icon alt text: tile name, falling back to the image path. */
  function iconAlt(cell: Extract<CellData, { kind: 'button' }>, src: string): string {
    return cell.name !== '' ? cell.name : src;
  }

  function removeSelf(event: Event): void {
    (event.currentTarget as HTMLElement).remove();
  }
</script>

{#each grid.folders as folder}
  <!-- NOTE: duplicate id="folder-X" on both divs is upstream behavior. -->
  <div id="folder-{folder.folderId}" class="buttons-center invisible">
    <div id="folder-{folder.folderId}" class="all-buttons">
      {#each folder.cells as cell (cell.editModalId)}
        {#if cell.kind === 'void'}
          <div class="void form-{cell.buttonId}" id={cell.editModalId}>
            <div class="checkbox" style="display: none;"></div>
            <div
              class="add-button"
              data-testid="add-slot"
              add_FOLDER={cell.folderId}
              add_ID={String(cell.buttonId)}
              style="display: none;"
              title={addLabel}
            >
              <PlusIcon />
            </div>
          </div>
        {:else}
          <form class="form-{cell.buttonId} form" id={cell.editModalId}>
            {#if cell.folderHandler !== null}
              <div
                class="swapMode-open-folder"
                use:stringAttrs={folderAttrs(cell)}
                style="display: none;"
              >
                {grid.openFolder}
              </div>
            {/if}
            <div class="container-editmode">
              <div
                class="edit-button"
                style="display: none;"
                edit_modal_ID={cell.editModalId}
                title={tileTitle(cell)}
              >
                <PencilIcon />
              </div>
              <div class="delete-button" style="display: none;" title={tileTitle(cell)}>
                <DeleteXIcon />
              </div>
            </div>
            <input type="hidden" class={'message ' + cell.dark} value={cell.hiddenValue} />
            <div class="checkbox" style="display: none;"></div>
            <!-- svelte-ignore a11y_no_redundant_roles: 1:1 port, upstream sets role="button". -->
            <button
              use:stringAttrs={folderAttrs(cell)}
              type={cell.folderHandler !== null ? undefined : 'submit'}
              id="button_{cell.editModalId}"
              edit_modal_ID={cell.editModalId}
              data-testid="deck-tile"
              data-message={cell.message}
              class={cell.cls}
              role="button"
              title={tileTitle(cell)}
              aria-label={tileTitle(cell)}
              style="overflow: hidden; max-height: 89.6px;"
            >
              {#if cell.media.kind === 'svg'}
                <SvgSlot slot={cell.media.slot} />
              {:else if cell.media.kind === 'img'}
                <!-- onerror removal mirrors the isfile guard for missing files. -->
                <img
                  src={cell.media.src}
                  draggable="false"
                  alt={iconAlt(cell, cell.media.src)}
                  onerror={removeSelf}
                  style="width: {cell.media.px}px; {cell.media.fill}"
                />
              {/if}
              {#if cell.showInsideName}
                <span class="buttontext-inside">{cell.name}</span>
              {/if}
              <!--|||||||||||||||||||||||-->
              {#if cell.usage !== null}
                <div class="usage">
                  <div class="usage-title {cell.usage.cls}" style={cell.usage.fill}>
                    {cell.usage.name}
                  </div>
                  <div class="usage-value {cell.usage.cls}" style={cell.usage.fill}>-</div>
                </div>
              {/if}
              <!--|||||||||||||||||||||||-->
            </button>
            {#if cell.showBelowName}
              <p class="buttontext" style={cell.namesColorStyle}>
                {cell.name}
              </p>
            {/if}
          </form>
          <EditModal
            ctx={ctx}
            editModalId={cell.editModalId}
            entry={cell.entry}
            message={cell.message}
          />
        {/if}
      {/each}
    </div>
  </div>
{/each}
