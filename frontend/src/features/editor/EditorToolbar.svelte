<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../../components/Icon.svelte';
  import Modal from '../../components/Modal.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { contract } from '../../lib/schema';
  import type { Editor } from './editor.svelte';
  import type { Folder } from '../../lib/contracts';
  const t = useTranslations();
  let {
    editing = true,
    editor,
    current,
    rootId,
    add,
    settings,
    remove,
    save,
    done,
  }: {
    editing?: boolean;
    editor: Editor;
    current: Folder;
    rootId: string;
    add: () => void;
    settings: () => void;
    remove: () => void;
    save: () => void;
    done: () => void;
  } = $props();
  const isFolder = $derived(current.id !== rootId && current.id !== 'home');
  const saveState = $derived(
    editor.saving
      ? 'saving'
      : editor.conflict || editor.failure
        ? 'error'
        : editor.dirty
          ? 'dirty'
          : 'saved',
  );
  const saveText = $derived(
    editor.saving
      ? t('ui_saving')
      : editor.conflict
        ? t('ui_conflict_draft_preserved')
        : editor.failure
          ? t('ui_save_failed_draft_preserved')
          : editor.dirty
            ? t('ui_unsaved_changes')
            : t('ui_all_changes_saved'),
  );
  let menuOpen = $state(false);
  let overflow = $state<HTMLDivElement>();
  let menu = $state<HTMLDivElement>();
  let menuButton = $state<HTMLButtonElement>();
  let renameButton = $state<HTMLButtonElement>();
  let nameInput = $state<HTMLInputElement>();
  let renaming = $state(false);
  let name = $state('');
  let nameError = $state(false);
  let deleting = $state(false);
  $effect(() => {
    current.id;
    editing;
    menuOpen = false;
    renaming = false;
    nameError = false;
    deleting = false;
  });
  async function beginRename() {
    if (!isFolder) return;
    menuOpen = false;
    name = current.label;
    nameError = false;
    renaming = true;
    await tick();
    nameInput?.focus();
    nameInput?.select();
  }
  function confirmRename(restoreFocus = false) {
    if (!renaming) return true;
    try {
      contract('Folder', { ...current, label: name });
    } catch {
      nameError = true;
      return false;
    }
    if (name !== current.label) editor.renameFolder(current.id, name);
    renaming = false;
    nameError = false;
    if (restoreFocus) void tick().then(() => renameButton?.focus());
    return true;
  }
  function cancelRename() {
    renaming = false;
    nameError = false;
    void tick().then(() => renameButton?.focus());
  }
  function run(action: () => void) {
    if (!confirmRename()) return;
    menuOpen = false;
    action();
  }
  async function openMenu(event?: KeyboardEvent) {
    if (!confirmRename()) return;
    if (event) event.preventDefault();
    menuOpen = !menuOpen;
    if (menuOpen) {
      await tick();
      menu?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
    }
  }
  function closeMenu(restoreFocus = false) {
    menuOpen = false;
    if (restoreFocus) menuButton?.focus();
  }
  function menuKeyboard(event: KeyboardEvent) {
    if (!menu) return;
    const items = [...menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closeMenu(true);
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      const next =
        event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? items.length - 1
            : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
      items[next]?.focus();
    } else if (event.key === 'Tab') closeMenu(true);
  }
  function outsideMenu(event: PointerEvent) {
    if (menuOpen && event.target instanceof Node && !overflow?.contains(event.target)) closeMenu();
  }
</script>

<svelte:window
  onpointerdown={outsideMenu}
  onbeforeunload={(event) => {
    if (renaming && name !== current.label) {
      event.preventDefault();
      event.returnValue = '';
    }
  }}
/>
<section class="editor-toolbar" aria-label={t('ui_deck_editor')}>
  <div class="editor-context">
    <Icon name={isFolder ? 'folder' : 'grid'} size={22} />
    <div class="editor-title" class:folder-context={isFolder}>
      {#if editing}<span class="context-label">{t(isFolder ? 'ui_folder' : 'ui_editing')}</span
        >{/if}
      {#if renaming}
        <div class="rename-row">
          <input
            bind:this={nameInput}
            bind:value={name}
            aria-label={t('ui_folder_name')}
            aria-invalid={nameError}
            aria-describedby={nameError ? 'folder-name-error' : undefined}
            oninput={() => (nameError = false)}
            onkeydown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                event.stopPropagation();
                confirmRename(true);
              }
              if (event.key === 'Escape') {
                event.preventDefault();
                event.stopPropagation();
                cancelRename();
              }
            }}
            onblur={(event) => {
              if (!(
                event.relatedTarget instanceof Element &&
                event.relatedTarget.closest('[data-rename-cancel]')
              ))
                confirmRename();
            }}
          />
          <button
            class="icon-button"
            aria-label={t('ui_confirm_folder_name')}
            onclick={() => confirmRename(true)}><Icon name="check" size={18} /></button
          >
          <button
            class="icon-button"
            data-rename-cancel
            aria-label={t('ui_cancel_rename')}
            onclick={cancelRename}><Icon name="close" size={18} /></button
          >
        </div>
        {#if nameError}<span id="folder-name-error" class="rename-error" role="alert"
            >{t('ui_invalid_folder_name')}</span
          >{/if}
      {:else}
        <div class="name-row">
          <strong class="folder-title" title={current.label}>{current.label}</strong>
          {#if editing && isFolder}<button
              bind:this={renameButton}
              class="icon-button rename-button"
              aria-label={t('ui_rename_folder')}
              onclick={beginRename}><Icon name="edit" size={16} /></button
            >{/if}
        </div>
      {/if}
    </div>
  </div>
  <div class="status-area">
    {#if editing || editor.dirty}<span
        class="save-status"
        data-state={saveState}
        role="status"
        aria-label={t('ui_save_status')}
        title={saveText}><span id="deck-save-status">{saveText}</span></span
      >{/if}
    {#if editing && editor.dirty}<button
        class="primary save-action"
        aria-describedby="deck-save-status"
        disabled={editor.saving}
        onclick={() => run(save)}>{t('ui_save_changes')}</button
      >{/if}
    {#if editing && editor.canUndoDeletion}<button
        class="icon-button"
        aria-label={t('ui_undo_deletion')}
        title={t('ui_removed_from_your_draft')}
        onclick={() => editor.undoDeletion()}><Icon name="back" size={18} /></button
      >{/if}
  </div>
  <div class="editor-actions">
    {#if editing}
      <div class="secondary-actions">
        <button
          class="icon-button"
          aria-label={t(isFolder ? 'ui_folder_settings' : 'ui_deck_settings')}
          onclick={() => run(settings)}><Icon name="settings" size={20} /></button
        >
        <div class="overflow" bind:this={overflow}>
          <button
            bind:this={menuButton}
            class="icon-button"
            aria-label={t(isFolder ? 'ui_folder_actions' : 'ui_deck_actions')}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            aria-controls={menuOpen ? 'editor-actions-menu' : undefined}
            onclick={() => openMenu()}
            onkeydown={(event) => {
              if (event.key === 'ArrowDown' || event.key === 'ArrowUp') void openMenu(event);
            }}><Icon name="more" size={20} /></button
          >
          {#if menuOpen}<div
              bind:this={menu}
              id="editor-actions-menu"
              class="actions-menu"
              role="menu"
              tabindex="-1"
              aria-label={t(isFolder ? 'ui_folder_actions' : 'ui_deck_actions')}
              onkeydown={menuKeyboard}
              onfocusout={(event) => {
                if (event.relatedTarget instanceof Node && !overflow?.contains(event.relatedTarget))
                  closeMenu();
              }}
            >
              <button role="menuitem" tabindex="-1" onclick={() => run(add)}
                ><Icon name="folder" size={18} />{t('ui_new_folder')}</button
              >
              {#if isFolder}<button
                  role="menuitem"
                  tabindex="-1"
                  class="danger"
                  onclick={() => {
                    closeMenu();
                    deleting = true;
                  }}><Icon name="trash" size={18} />{t('ui_delete_folder')}</button
                >{/if}
            </div>{/if}
        </div>
      </div>
      <button
        class="primary done"
        aria-describedby={editor.saving ? 'deck-save-status' : undefined}
        disabled={editor.saving}
        onclick={() => run(done)}>{t('ui_done')}</button
      >
    {:else}<button class="done" onclick={done}><Icon name="edit" size={18} />{t('ui_edit')}</button
      >{/if}
  </div>
</section>
{#if deleting && isFolder}
  <Modal
    label={t('ui_delete_folder')}
    close={() => {
      deleting = false;
      menuButton?.focus();
    }}
  >
    <h2>{t('ui_delete_folder')}</h2>
    <p>{t('ui_delete_folder_help', { label: current.label })}</p>
    <button
      onclick={() => {
        deleting = false;
        menuButton?.focus();
      }}>{t('ui_cancel')}</button
    >
    <button
      class="danger"
      onclick={() => {
        deleting = false;
        remove();
      }}>{t('ui_delete_folder')}</button
    >
  </Modal>
{/if}

<style>
  .editor-toolbar {
    position: fixed;
    inset: auto 12px 12px;
    z-index: 20;
    max-width: none;
    margin: 0;
    min-height: 64px;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 16px;
    background: #1c1f2aeb;
    backdrop-filter: blur(16px);
    border: 1px solid #ffffff20;
    border-radius: 14px;
  }
  .editor-context,
  .name-row,
  .rename-row,
  .status-area,
  .editor-actions,
  .secondary-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .editor-context {
    flex: 1;
  }
  .editor-title {
    min-width: 0;
  }
  .folder-context {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .folder-context:has(.rename-row) {
    display: block;
  }
  .folder-context:has(.rename-row) .context-label {
    display: none;
  }
  .folder-context .context-label {
    flex-shrink: 0;
  }
  .context-label {
    display: block;
    font-size: 0.7rem;
    color: var(--text-muted);
    line-height: 1.3;
  }
  .folder-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.9rem;
  }
  .status-area {
    flex: 0 1 auto;
    max-width: 45%;
  }
  .save-status {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 0.75rem;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .save-status > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .save-status::before {
    content: '';
    display: block;
    flex-shrink: 0;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
  }
  .save-status[data-state='saved']::before {
    background: #71c99a;
  }
  .save-status[data-state='dirty']::before {
    background: #e8bb65;
  }
  .save-status[data-state='error']::before {
    background: #f28b94;
  }
  .editor-toolbar button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 44px;
    flex-shrink: 0;
    padding: 8px 12px;
    font-size: 0.85rem;
  }
  .editor-toolbar .icon-button {
    width: 44px;
    padding: 8px;
    background: transparent;
    border-color: transparent;
  }
  .editor-toolbar .icon-button:hover {
    background: var(--surface-hover);
  }
  .rename-row input {
    min-width: 0;
    width: min(220px, 100%);
    min-height: 44px;
  }
  .rename-error {
    display: block;
    font-size: 0.7rem;
    color: #f28b94;
  }
  .overflow {
    position: relative;
  }
  .actions-menu {
    position: absolute;
    bottom: calc(100% + 12px);
    right: 0;
    width: 210px;
    max-width: calc(100vw - 24px);
    padding: 6px;
    background: var(--surface);
    border: 1px solid #ffffff25;
    border-radius: 12px;
    box-shadow: 0 8px 24px #0004;
  }
  .actions-menu button {
    display: flex;
    width: 100%;
    justify-content: flex-start;
    border: 0;
    background: transparent;
  }
  .actions-menu button:hover,
  .actions-menu button:focus-visible {
    background: var(--surface-hover);
  }
  @media (max-width: 640px) {
    .editor-toolbar {
      inset: auto 8px max(8px, env(safe-area-inset-bottom));
      padding: 6px 8px;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 0 6px;
    }
    .editor-context {
      grid-area: 1 / 1;
    }
    .context-label {
      font-size: 0.65rem;
    }
    .folder-title {
      font-size: 0.8rem;
    }
    .status-area {
      grid-area: 2 / 1;
      max-width: none;
      min-height: 44px;
      gap: 4px;
    }
    .save-status {
      flex: 1;
      font-size: 0.65rem;
    }
    .editor-actions {
      display: contents;
    }
    .secondary-actions {
      grid-area: 2 / 2;
      gap: 0;
    }
    .done {
      grid-area: 1 / 2;
      justify-self: end;
    }
    .editor-toolbar button {
      font-size: 0.75rem;
      padding: 8px;
    }
    .rename-row {
      gap: 0;
    }
    .rename-row input {
      font-size: 0.8rem;
      padding: 6px;
    }
    .editor-context:has(.rename-row) {
      grid-column: 1 / -1;
      padding-right: 60px;
    }
  }
</style>
