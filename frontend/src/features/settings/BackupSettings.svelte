<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import SettingsCard from './ui/SettingsCard.svelte';
  import FileField from './ui/FileField.svelte';
  import { onDestroy } from 'svelte';
  import type { Editor } from '../editor/editor.svelte';
  import type { Config } from '../../lib/contracts';
  import { contract } from '../../lib/schema';
  let {
    editor,
    tutorialPreview = false,
    reload,
    notify,
  }: {
    editor: Editor;
    tutorialPreview?: boolean;
    reload: () => Promise<void>;
    notify: (message: string) => void;
  } = $props();
  let preview = $state<Config | null>(null),
    error = $state('');
  const shownPreview = $derived(tutorialPreview ? editor.draft : preview);
  let inspection = 0;
  onDestroy(() => {
    inspection++;
  });
  async function inspect(event: Event) {
    const generation = ++inspection;
    preview = null;
    error = '';
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.size > 16 * 1024 * 1024) {
      error = 'This backup exceeds the 16 MB restore limit. Your draft has not changed.';
      return;
    }
    try {
      const candidate = contract<Config>('Config', JSON.parse(await file.text()));
      if (generation === inspection) preview = candidate;
    } catch {
      if (generation !== inspection) return;
      error = 'This file is not a valid canonical v2 backup. Your draft has not changed.';
    }
  }
  function apply() {
    if (tutorialPreview || !preview) return;
    if (editor.dirty && !window.confirm(t('Replace unsaved changes with this backup?'))) return;
    editor.restore(preview);
    preview = null;
    notify('Backup loaded into your draft. Save to apply.');
  }
  function download() {
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(editor.draft, null, 2)], { type: 'application/json' }),
    );
    const link = document.createElement('a');
    link.href = url;
    link.download = 'webdeck-v2-config.json';
    link.click();
    // Delay revocation until the browser has observed the download navigation.
    requestAnimationFrame(() => URL.revokeObjectURL(url));
  }
</script>

<div id="settings-backups" class="settings-page" aria-labelledby="backups-title">
  <div class="settings-page-heading"><h2 id="backups-title">{t('ui_backups')}</h2></div>
  <SettingsCard title={t('ui_backup_restore')} icon="folder">
    <p>{t('ui_restore_loads_a_draft_save_changes_to_apply_it_to_the_host')}</p>
    <div data-guide="backup-import">
      <FileField label={t('ui_restore_a_v2_backup')} accept=".json" onchange={inspect} />
    </div>
    {#if error}<p role="alert">{t(error)}</p>{/if}
    {#if shownPreview}<div class="backup-preview">
        <p>
          {t('ui_backup_counts', {
            folders: shownPreview.layout.folders.length,
            buttons: shownPreview.layout.folders.reduce(
              (count, folder) => count + folder.buttons.length,
              0,
            ),
          })}
        </p>
        <button onclick={apply}>{t('ui_apply_backup_to_draft')}</button><button
          onclick={() => {
            if (!tutorialPreview) {
              inspection++;
              preview = null;
            }
          }}>{t('ui_cancel_restore')}</button
        >
      </div>{/if}
  </SettingsCard>
  <SettingsCard title={t('ui_backup_export')} icon="copy">
    <p>{t('ui_your_unsaved_draft_remains_available_after_a_save_conflict')}</p>
    <div class="row">
      <button data-guide="backup-export" onclick={download}>{t('ui_download_backup')}</button
      ><button
        aria-describedby={editor.saving ? 'settings-save-status' : undefined}
        disabled={editor.saving}
        data-guide="backup-reload"
        onclick={reload}>{t('ui_reload_configuration')}</button
      >
    </div>
  </SettingsCard>
</div>

<style>
  .backup-preview {
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
  }
</style>
