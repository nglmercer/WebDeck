<script lang="ts">
  import FilePicker from '../../components/FilePicker.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    guideSource,
    imageUrl = $bindable(''),
    imagePath = $bindable(''),
    liveImage = $bindable(false),
    busy,
    filename = '',
    metadata = '',
    preview,
    upload,
    importUrl,
    importLocal,
    chooseLocal,
  }: {
    imageUrl: string;
    guideSource?: 'url' | 'local' | null | undefined;
    imagePath: string;
    liveImage: boolean;
    busy: boolean;
    filename?: string;
    metadata?: string;
    preview?: string | undefined;
    upload: (file: File) => Promise<void>;
    importUrl: () => void;
    importLocal: () => void;
    chooseLocal: () => void;
  } = $props();
  let source = $state<'url' | 'local' | null>(null);
  $effect(() => {
    if (guideSource !== undefined) source = guideSource;
  });
</script>

<div class="image-sources">
  <FilePicker
    label={t('editor.visualImage')}
    accept=".png,.jpg,.jpeg,.webp,.gif,.svg"
    {filename}
    {metadata}
    {preview}
    {busy}
    onselect={upload}
  />
  <div class="source-actions">
    <button
      type="button"
      class="text-button"
      disabled={busy}
      aria-expanded={source === 'local'}
      onclick={() => (source = source === 'local' ? null : 'local')}>{t('common.localFile')}</button
    >
    <button
      type="button"
      class="text-button"
      aria-expanded={source === 'url'}
      disabled={busy}
      onclick={() => (source = source === 'url' ? null : 'url')}
      >{t('images.imageImportUrl')}</button
    >
  </div>
  {#if source === 'url'}
    <div class="url-source">
      <label>{t('images.imageUrl')}<input type="url" bind:value={imageUrl} disabled={busy} /></label
      >
      <label class="check"
        ><input type="checkbox" bind:checked={liveImage} disabled={busy} />{t(
          'images.liveImage',
        )}</label
      >
      <button type="button" class="primary" disabled={busy || !imageUrl.trim()} onclick={importUrl}
        >{t('images.importImageUrl')}</button
      >
    </div>
  {:else if source === 'local'}
    <div class="url-source">
      <label>{t('images.localImagePath')}<input bind:value={imagePath} disabled={busy} /></label>
      <div class="local-actions">
        <button type="button" disabled={busy} onclick={chooseLocal}
          >{t('images.chooseLocalImage')}</button
        ><button
          type="button"
          class="primary"
          disabled={busy || !imagePath.trim()}
          onclick={importLocal}>{t('editor.importLocalImage')}</button
        >
      </div>
    </div>
  {/if}
</div>

<style>
  .source-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 8px;
  }
  .text-button {
    border: 0;
    padding: 4px 0;
    background: transparent;
    color: var(--accent);
    font-size: 0.75rem;
  }
  .url-source {
    margin-top: 12px;
    padding: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
  }
  .url-source label:first-child {
    margin-top: 0;
  }
  .local-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
  .local-actions button {
    font-size: 0.8rem;
  }
  .url-source > button {
    display: block;
    margin-left: auto;
    font-size: 0.8rem;
  }
</style>
