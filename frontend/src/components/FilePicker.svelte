<script lang="ts">
  import Icon from './Icon.svelte';
  import { useTranslations } from '../lib/i18n';
  const t = useTranslations();
  let {
    label,
    accept,
    filename = '',
    metadata = '',
    preview,
    icon = 'image',
    busy = false,
    onselect,
  }: {
    label: string;
    accept: string;
    filename?: string;
    metadata?: string;
    preview?: string | undefined;
    icon?: string;
    busy?: boolean;
    onselect: (file: File) => void | Promise<void>;
  } = $props();
  let input: HTMLInputElement;
  let dragging = $state(false);
  let error = $state('');
  let pending = $state(false);
  const disabled = $derived(busy || pending);
  async function choose(files: FileList | null) {
    if (disabled || !files?.length) return;
    error = '';
    if (files.length !== 1) {
      error = t('files.one');
      return;
    }
    const file = files[0]!;
    const matches = accept.split(',').some((item) => {
      const rule = item.trim();
      return rule.startsWith('.')
        ? file.name.toLowerCase().endsWith(rule.toLowerCase())
        : rule.endsWith('/*')
          ? file.type.startsWith(rule.slice(0, -1))
          : file.type === rule;
    });
    if (!matches) {
      error = t('files.type');
      return;
    }
    if (file.size > 16 * 1024 * 1024) {
      error = t('files.size');
      return;
    }
    pending = true;
    try {
      await onselect(file);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      pending = false;
    }
  }
</script>

<div class="file-picker">
  <span class="label">{label}</span>
  <div
    class="drop-area"
    class:dragging
    class:busy={disabled}
    role="group"
    aria-label={label}
    aria-busy={disabled}
    ondragover={(event) => {
      event.preventDefault();
      if (!disabled) dragging = true;
    }}
    ondragleave={(event) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) dragging = false;
    }}
    ondrop={(event) => {
      event.preventDefault();
      dragging = false;
      void choose(event.dataTransfer?.files ?? null);
    }}
  >
    <span class="file-visual"
      >{#if preview}<img src={preview} alt="" />{:else}<Icon name={icon} size={30} />{/if}</span
    >
    <span class="file-copy"
      ><strong title={filename || undefined}
        >{disabled ? t('files.loading') : filename || t('files.drop')}</strong
      >{#if metadata}<small>{metadata}</small>{/if}</span
    >
    <button type="button" {disabled} onclick={() => input.click()}
      >{t(filename ? 'files.change' : 'editor.chooseFile')}</button
    >
    <input
      bind:this={input}
      class="sr-only"
      tabindex="-1"
      type="file"
      {accept}
      {disabled}
      aria-label={label}
      onchange={(event) => {
        void choose(event.currentTarget.files);
        event.currentTarget.value = '';
      }}
    />
  </div>
  {#if error}<p role="alert">{t.message(error)}</p>{/if}
</div>

<style>
  .file-picker {
    min-width: 0;
  }
  .label {
    display: block;
    margin-bottom: 8px;
    font-size: 0.85rem;
    font-weight: 550;
  }
  .drop-area {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px;
    min-height: 80px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-panel);
    background: var(--canvas);
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .drop-area.dragging {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .drop-area:focus-within {
    border-color: var(--accent);
  }
  .file-visual {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    flex: 0 0 48px;
    color: var(--text-muted);
    overflow: hidden;
    border-radius: 6px;
  }
  .file-visual img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .file-copy {
    min-width: 0;
    flex: 1;
  }
  strong {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.85rem;
    font-weight: 550;
  }
  small {
    display: block;
    color: var(--text-muted);
    margin-top: 5px;
    font-size: 0.75rem;
    overflow-wrap: anywhere;
  }
  button {
    flex-shrink: 0;
    font-size: 0.8rem;
    padding: 8px 12px;
  }
  p {
    color: var(--error-text);
    font-size: 0.8rem;
    margin: 8px 0 0;
  }
  .busy {
    opacity: 0.7;
  }
  @media (max-width: 420px) {
    .drop-area {
      gap: 10px;
      padding: 12px;
      flex-wrap: wrap;
    }
    .file-visual {
      flex-basis: 40px;
      width: 40px;
      height: 40px;
    }
    .file-copy {
      flex-basis: calc(100% - 60px);
    }
    button {
      margin-left: auto;
    }
  }
</style>
