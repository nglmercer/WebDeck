<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Editor } from './editor.svelte';
  import type { Folder } from '../../lib/contracts';
  let {
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
      <h2>{t('ui_editing_folder', { label: current.label })}</h2>
      <span class="save-status" aria-label={t('ui_save_status')}
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
      </span>
    </div>
    <div class="row">
      <button
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
    </div>
  </div>
  {#if editor.canUndoDeletion}<div class="row">
      <span>{t('ui_removed_from_your_draft')}</span><button onclick={() => editor.undoDeletion()}
        >{t('ui_undo_deletion')}</button
      >
    </div>{/if}
  <div class="toolbar">
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
    <button onclick={addButton}>{t('ui_add_button')}</button>
    <button onclick={add}>{t('ui_add_folder')}</button>
    <button onclick={settings}>{t('ui_settings')}</button>
    <button class="danger" onclick={remove}>{t('ui_delete_folder')}</button>
  </div>
</section>

<style>
  .editor-toolbar {
    max-width: none;
    margin-bottom: 16px;
  }
  .editor-toolbar .heading {
    margin-bottom: 12px;
  }
  .editor-toolbar h2 {
    margin: 0 0 4px;
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
  }
</style>
