<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    imageUrl = $bindable(''),
    imagePath = $bindable(''),
    liveImage = $bindable(false),
    busy,
    upload,
    importUrl,
    importLocal,
    chooseLocal,
  }: {
    imageUrl: string;
    imagePath: string;
    liveImage: boolean;
    busy: boolean;
    upload: (event: Event) => void;
    importUrl: () => void;
    importLocal: () => void;
    chooseLocal: () => void;
  } = $props();
  let fileInput = $state<HTMLInputElement>();
  let source = $state('upload');
</script>

<div class="image-sources" aria-busy={busy}>
  <div class="source-switch" role="group" aria-label={t('ui_image_source')}>
    {#each ['upload', 'url', 'local'] as item}
      <button type="button" aria-pressed={source === item} onclick={() => (source = item)}>
        {t(
          item === 'upload' ? 'ui_upload_image' : item === 'url' ? 'ui_image_url' : 'ui_local_file',
        )}
      </button>
    {/each}
  </div>
  <div class="source-body">
    {#if source === 'upload'}
      <input
        bind:this={fileInput}
        class="sr-only"
        tabindex="-1"
        type="file"
        accept="image/*"
        aria-label={t('ui_upload_image')}
        disabled={busy}
        onchange={upload}
      />
      <button type="button" class="upload-card" disabled={busy} onclick={() => fileInput?.click()}>
        <span class="upload-icon"><Icon name="image" /></span>
        <strong>{t('ui_choose_image')}</strong>
        <span>{t('ui_upload_image_help')}</span>
      </button>
    {:else if source === 'url'}
      <label
        >{t('ui_image_url')}<input
          type="text"
          inputmode="url"
          bind:value={imageUrl}
          disabled={busy}
        /></label
      >
      <label class="check"
        ><input type="checkbox" bind:checked={liveImage} disabled={busy} />{t(
          'ui_live_image',
        )}</label
      >
      <div class="source-actions">
        <button
          type="button"
          class="primary"
          disabled={busy || !imageUrl.trim()}
          onclick={importUrl}>{t('ui_import_image_url')}</button
        >
      </div>
    {:else}
      <label>{t('ui_local_image_path')}<input bind:value={imagePath} disabled={busy} /></label>
      <div class="source-actions">
        <button type="button" disabled={busy} onclick={chooseLocal}
          >{t('ui_choose_local_image')}</button
        >
        <button
          type="button"
          class="primary"
          disabled={busy || !imagePath.trim()}
          onclick={importLocal}>{t('ui_import_local_image')}</button
        >
      </div>
    {/if}
    {#if busy}<p class="status" role="status">{t('ui_importing_image')}</p>{/if}
  </div>
  <p class="help">{t('ui_image_import_help')}</p>
</div>

<style>
  .image-sources {
    margin-top: 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-panel);
    overflow: hidden;
  }
  .source-switch {
    display: flex;
    gap: 4px;
    padding: 6px;
    background: var(--canvas);
    border-bottom: 1px solid var(--border-subtle);
  }
  .source-switch button {
    flex: 1;
    min-width: 0;
    padding: 9px 6px;
    font-size: 0.8rem;
    border-color: transparent;
    background: transparent;
    color: var(--text-muted);
  }
  .source-switch button[aria-pressed='true'] {
    background: var(--surface-raised);
    border-color: var(--border);
    color: var(--text);
  }
  .source-body {
    padding: 14px;
  }
  .source-body label {
    margin: 0 0 12px;
  }
  .upload-card {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 20px 12px;
    background: var(--canvas);
    border: 1px dashed var(--border);
  }
  .upload-card:hover {
    border-color: var(--accent);
    background: var(--surface-hover);
  }
  .upload-icon {
    color: var(--accent);
  }
  .upload-card > span:last-child {
    color: var(--text-muted);
    font-size: 0.8rem;
  }
  .source-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
  }
  .source-actions button {
    font-size: 0.85rem;
  }
  .help {
    margin: 0;
    padding: 12px 14px;
    border-top: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: 0.78rem;
    line-height: 1.5;
  }
  .status {
    margin: 12px 0 0;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
</style>
