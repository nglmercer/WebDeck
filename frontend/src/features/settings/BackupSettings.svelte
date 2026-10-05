<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onDestroy } from 'svelte';
  import type { Editor } from '../editor/editor.svelte';
  import type { Config } from '../../lib/contracts';
  import { contract } from '../../lib/schema';
  let {
    editor,
    reload,
    notify,
  }: { editor: Editor; reload: () => Promise<void>; notify: (message: string) => void } = $props();
  let preview = $state<Config | null>(null),
    filename = $state(''),
    error = $state('');
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
    filename = file.name;
    try {
      const candidate = contract<Config>('Config', JSON.parse(await file.text()));
      if (generation === inspection) preview = candidate;
    } catch {
      if (generation !== inspection) return;
      error = 'This file is not a valid canonical v2 backup. Your draft has not changed.';
    }
  }
  function apply() {
    if (!preview) return;
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

<section id="settings-backups" aria-labelledby="backups-title">
  <h2 id="backups-title">{t('ui_backups')}</h2>
  <p>{t('ui_restore_loads_a_draft_save_changes_to_apply_it_to_the_host')}</p>
  <label>{t('ui_restore_a_v2_backup')}<input type="file" accept=".json" onchange={inspect} /></label
  >
  {#if error}<p role="alert">{t(error)}</p>{/if}
  {#if preview}<div class="backup-preview">
      <strong>{filename}</strong>
      <p>
        {t('ui_backup_counts', {
          folders: preview.layout.folders.length,
          buttons: preview.layout.folders.reduce(
            (count, folder) => count + folder.buttons.length,
            0,
          ),
        })}
      </p>
      <button onclick={apply}>{t('ui_apply_backup_to_draft')}</button><button
        onclick={() => {
          inspection++;
          preview = null;
        }}>{t('ui_cancel_restore')}</button
      >
    </div>{/if}
  <p>{t('ui_your_unsaved_draft_remains_available_after_a_save_conflict')}</p>
  <div class="row">
    <button onclick={download}>{t('ui_download_backup')}</button><button
      aria-describedby={editor.saving ? 'settings-save-status' : undefined}
      disabled={editor.saving}
      onclick={reload}>{t('ui_reload_configuration')}</button
    >
  </div>
</section>

<style>
  .backup-preview {
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
  }
</style>
