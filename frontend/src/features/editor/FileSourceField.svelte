<script lang="ts">
  import FilePicker from '../../components/FilePicker.svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { upload, select } from '../../lib/api/client';
  import type { FileSource } from '../../lib/contracts';
  import { fileSize, type MediaMetadata } from './audio';
  const t = useTranslations();
  let {
    label,
    hideLabel = false,
    value,
    onchange,
    accept = '',
    maxSize = 16 * 1024 * 1024,
    metadata,
    onmetadata,
  }: {
    label: string;
    hideLabel?: boolean;
    value: FileSource;
    onchange: (source: FileSource) => void;
    accept?: string;
    maxSize?: number;
    metadata?: MediaMetadata | undefined;
    onmetadata?: ((metadata: MediaMetadata) => void) | undefined;
  } = $props();
  let filename = $state(''),
    size = $state(0),
    manual = $state(false),
    busy = $state(false),
    error = $state('');
  const source = $derived(value.type === 'asset' ? value.id : value.path);
  const displayName = $derived(metadata?.source === source ? metadata.name : filename);
  const displaySize = $derived(metadata?.source === source ? metadata.size : size);
  let previous = '';
  $effect(() => {
    if (source !== previous) {
      previous = source;
      filename = source.split(/[\\/]/).at(-1) ?? source;
      size = 0;
      if (source) error = '';
    }
  });
  async function file(file: File) {
    error = '';
    const next = await upload(file);
    previous = next.type === 'asset' ? next.id : next.path;
    filename = file.name;
    size = file.size;
    onmetadata?.({ source: previous, name: file.name, size: file.size });
    onchange(next);
  }
  async function local() {
    busy = true;
    error = '';
    try {
      const result = await select('file');
      if (result.source) onchange(result.source);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="source-file">
  <FilePicker
    {label}
    {hideLabel}
    {accept}
    {maxSize}
    filename={source ? displayName : ''}
    metadata={source
      ? displaySize
        ? fileSize(displaySize)
        : t(value.type === 'asset' ? 'forms.source.uploaded' : 'forms.source.local')
      : ''}
    icon="file-code"
    {busy}
    onselect={file}
  />
  <div class="source-actions">
    <button type="button" class="text-button" disabled={busy} onclick={local}
      ><Icon name="folder" size={17} />{t('forms.source.chooseLocal')}</button
    ><button
      type="button"
      class="text-button"
      aria-expanded={manual}
      onclick={() => (manual = !manual)}
      ><Icon name="link" size={16} />{t('forms.source.serverPath')}</button
    >{#if source}<button
        type="button"
        class="text-button remove"
        aria-label={t('forms.source.remove')}
        onclick={() => {
          filename = '';
          size = 0;
          onchange({ type: 'asset', id: '' });
        }}><Icon name="close" size={18} /></button
      >{/if}
  </div>
  {#if manual}<label
      >{t('forms.source.serverPath')}<input
        value={value.type === 'external' ? value.path : ''}
        placeholder={t('forms.source.pathPlaceholder')}
        maxlength="4096"
        oninput={(event) => onchange({ type: 'external', path: event.currentTarget.value })}
      /></label
    >{/if}
  <input
    class="validity-proxy"
    tabindex="-1"
    aria-hidden="true"
    required
    value={source}
    oninvalid={(event) => {
      event.preventDefault();
      error = 'forms.source.required';
    }}
  />
  {#if error}<p role="alert">{t.message(error)}</p>{/if}
</div>

<style>
  .source-file {
    position: relative;
    min-width: 0;
  }
  .source-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  .text-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 44px;
    background: transparent;
    border-color: transparent;
    color: var(--accent);
    font-size: 0.78rem;
    padding: 6px;
  }
  .remove {
    margin-left: auto;
  }
  .validity-proxy {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
    padding: 0;
    border: 0;
  }
  p {
    font-size: 0.75rem;
    color: var(--error-text);
    margin: 4px 0;
  }
</style>
