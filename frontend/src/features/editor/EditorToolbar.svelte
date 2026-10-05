<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let expanded = $state(false);

  import type { Editor } from './editor.svelte';
  import type { Folder } from '../../lib/contracts';
  let {
    editing = true,
    editor,
    current,
    active,
    navigate,
    add,
    addButton,
    settings,
    remove,
    save,
    done,
  }: {
    editing?: boolean;
    editor: Editor;
    current: Folder;
    active: string;
    navigate: (id: string) => void;
    add: () => void;
    addButton: () => void;
    settings: () => void;
    remove: () => void;
    save: () => void;
    done: () => void;
  } = $props();
</script>

<section class="editor-toolbar" aria-label={t('ui_deck_editor')}>
  <div class="heading">
    <div>
      <p class="folder-title">
        {editing ? t('ui_editing_folder', { label: current.label }) : current.label}
      </p>
      {#if editing || editor.dirty}<span class="save-status" aria-label={t('ui_save_status')}
          ><span id="deck-save-status"
            >{editor.saving
              ? t('ui_saving')
              : editor.conflict
                ? t('ui_conflict_draft_preserved')
                : editor.failure
                  ? t('ui_save_failed_draft_preserved')
                  : editor.dirty
                    ? t('ui_unsaved_changes')
                    : t('ui_all_changes_saved')}</span
          >
        </span>{/if}
    </div>
    <div class="row">
      {#if editing}<button
          aria-describedby="deck-save-status"
          class="primary"
          disabled={!editor.dirty || editor.saving}
          onclick={save}>{t('ui_save_changes')}</button
        >
        <button
          aria-describedby={editor.saving ? 'deck-save-status' : undefined}
          disabled={editor.saving}
          onclick={done}>{t('ui_done')}</button
        >
        <button
          class="tools-toggle"
          aria-label={t('ui_folder_tools')}
          aria-expanded={expanded}
          aria-controls="folder-tools"
          onclick={() => (expanded = !expanded)}><Icon name="settings" size={18} /></button
        >
      {:else}<button onclick={done}><Icon name="edit" size={18} />{t('ui_edit')}</button>{/if}
    </div>
  </div>
  {#if editing && editor.canUndoDeletion}<div class="row">
      <span>{t('ui_removed_from_your_draft')}</span><button onclick={() => editor.undoDeletion()}
        >{t('ui_undo_deletion')}</button
      >
    </div>{/if}
  {#if editing}<div id="folder-tools" class="toolbar" class:expanded>
      <label
        >{t('ui_folder')}<select
          value={active}
          onchange={(event) => navigate(event.currentTarget.value)}
        >
          {#each editor.draft.layout.folders as folder}<option value={folder.id}
              >{folder.label}</option
            >{/each}
        </select></label
      >
      <label
        >{t('ui_folder_name')}<input
          value={current.label}
          oninput={(event) => editor.renameFolder(current.id, event.currentTarget.value)}
        /></label
      >
      <button onclick={addButton}><Icon name="plus" size={18} />{t('ui_add_button')}</button>
      <button onclick={add}><Icon name="folder" size={18} />{t('ui_add_folder')}</button>
      <button onclick={settings}><Icon name="settings" size={18} />{t('ui_settings')}</button>
      <button class="danger" onclick={remove}
        ><Icon name="trash" size={18} />{t('ui_delete_folder')}</button
      >
    </div>{/if}
</section>

<style>
  .editor-toolbar :global(button) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .editor-toolbar {
    max-width: none;
    position: fixed;
    bottom: 12px;
    left: 12px;
    right: 12px;
    z-index: 20;
    padding: 12px 16px;
    background: #1c1f2af2;
    backdrop-filter: blur(20px);
    border: 1px solid #ffffff20;
    border-radius: 16px;
    box-shadow: 0 12px 40px #0006;
    max-height: calc(100dvh - 24px);
  }
  .tools-toggle {
    display: none !important;
  }
  .editor-toolbar .heading {
    margin-bottom: 8px;
  }
  .editor-toolbar .folder-title {
    margin: 0;
    font-weight: 600;
  }
  .editor-toolbar .save-status {
    color: var(--text-muted);
    font-size: 0.85rem;
  }
  .editor-toolbar .toolbar {
    padding: 0;
    align-items: end;
  }
  .editor-toolbar .toolbar label {
    margin: 0;
    min-width: 130px;
    flex: 1;
    max-width: 240px;
  }
  @media (max-width: 640px), (pointer: coarse) {
    .editor-toolbar {
      bottom: max(8px, env(safe-area-inset-bottom));
      left: 8px;
      right: 8px;
      padding: 10px 12px;
    }
    .editor-toolbar .heading {
      flex-wrap: nowrap;
      gap: 8px;
      margin: 0;
    }
    .heading > div:first-child {
      min-width: 0;
    }
    .folder-title {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 0.85rem;
    }
    .editor-toolbar .save-status {
      font-size: 0.7rem;
    }
    .heading .row {
      gap: 6px;
      flex-wrap: nowrap;
    }
    .heading :global(button) {
      padding: 8px;
      font-size: 0.8rem;
    }
    .tools-toggle {
      display: inline-flex !important;
    }
    .editor-toolbar .toolbar {
      display: none;
    }
    .editor-toolbar .toolbar.expanded {
      display: flex;
      margin-top: 12px;
      max-height: 40dvh;
      overflow-y: auto;
      overscroll-behavior: contain;
    }
  }
</style>
