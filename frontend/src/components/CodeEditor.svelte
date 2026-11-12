<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { id } from '../lib/id';
  import { useTranslations } from '../lib/i18n';
  import type { mountEditor } from './code/editor';
  const t = useTranslations(),
    uid = `code-${id()}`;
  let {
    label,
    value,
    onchange,
    language = 'text',
    error = '',
    maxLength = 65536,
    compact = false,
    hideLabel = false,
  }: {
    label: string;
    value: string;
    onchange: (value: string) => void;
    language?: string;
    error?: string;
    maxLength?: number;
    compact?: boolean;
    hideLabel?: boolean;
  } = $props();
  let host: HTMLDivElement, proxy: HTMLTextAreaElement;
  let editor = $state.raw<ReturnType<typeof mountEditor> | undefined>();
  let expanded = $state(false),
    copied = $state(false),
    copyError = $state(false);
  let copyingTimer: ReturnType<typeof setTimeout>;
  let sent = '';
  function input(text: string) {
    sent = text;
    onchange(text);
  }
  onMount(() => {
    let cancelled = false;
    void import('./code/editor')
      .then(({ mountEditor }) => {
        if (cancelled) return;
        editor = mountEditor(host, value, language, label, input, compact);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      clearTimeout(copyingTimer);
      editor?.destroy();
    };
  });
  $effect(() => {
    if (editor && editor.view.state.doc.toString() !== value && value !== sent)
      editor.setValue(value);
  });
  $effect(() => {
    editor?.setLanguage(language);
  });
  $effect(() => {
    const message =
      error || (value.length > maxLength ? t('forms.code.tooLong', { max: maxLength }) : '');
    proxy?.setCustomValidity(message);
    if (editor) {
      editor.view.contentDOM.setAttribute('aria-label', label);
      editor.view.contentDOM.setAttribute('aria-invalid', String(!!message));
      editor.view.contentDOM.setAttribute('aria-describedby', message ? uid + '-error' : '');
    }
  });
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      copied = true;
      copyError = false;
      clearTimeout(copyingTimer);
      copyingTimer = setTimeout(() => (copied = false), 1800);
    } catch {
      copyError = true;
    }
  }
</script>

<div class="code-field" class:expanded class:compact>
  <div class="toolbar">
    <span class="code-label" class:sr-only={hideLabel}>{label}</span><span class="language"
      >{language === 'javascript'
        ? 'JavaScript'
        : language === 'json'
          ? 'JSON'
          : language === 'shell'
            ? 'Shell'
            : t('forms.code.text')}</span
    >{#if !compact}<button
        type="button"
        aria-label={t(copied ? 'forms.code.copied' : 'forms.code.copy')}
        title={t('forms.code.copy')}
        onclick={copy}><Icon name={copied ? 'check' : 'copy'} size={17} /></button
      ><button
        type="button"
        aria-label={t(expanded ? 'forms.code.collapse' : 'forms.code.expand')}
        aria-pressed={expanded}
        onclick={() => {
          expanded = !expanded;
          editor?.view.requestMeasure();
        }}><Icon name="fullscreen" size={17} /></button
      >{/if}
  </div>
  <div class="code-surface" bind:this={host}></div>
  <textarea
    bind:this={proxy}
    id={editor ? undefined : uid}
    class:proxy={!!editor}
    {value}
    maxlength={maxLength}
    aria-label={editor ? undefined : label}
    aria-hidden={editor ? true : undefined}
    tabindex={editor ? -1 : undefined}
    aria-invalid={!!error}
    aria-describedby={error ? uid + '-error' : undefined}
    rows={compact ? 2 : 8}
    spellcheck="false"
    oninput={(event) => input(event.currentTarget.value)}
    oninvalid={(event) => {
      if (editor) {
        event.preventDefault();
        editor.view.focus();
      }
    }}></textarea>
  {#if error || value.length > maxLength}<p id={uid + '-error'} role="alert">
      {error || t('forms.code.tooLong', { max: maxLength })}
    </p>{/if}
  {#if copyError}<p role="status">{t('forms.code.copyFailed')}</p>{/if}
</div>

<style>
  .code-field {
    position: relative;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--canvas);
  }
  .toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    min-height: 44px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .toolbar .code-label {
    margin: 0;
    flex: 1;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .language {
    margin-left: auto;
    color: var(--text);
    font:
      500 0.7rem ui-monospace,
      monospace;
    padding: 3px 6px;
    background: var(--accent-surface);
    border-radius: 4px;
  }
  .toolbar button {
    display: grid;
    place-items: center;
    width: 36px;
    min-height: 36px;
    padding: 6px;
    background: transparent;
    border-color: transparent;
  }
  .code-field:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .code-surface {
    min-width: 0;
  }
  .proxy {
    position: absolute !important;
    width: 1px !important;
    height: 1px !important;
    min-height: 0 !important;
    opacity: 0;
    pointer-events: none;
    padding: 0 !important;
    border: 0 !important;
  }
  textarea {
    display: block;
    width: 100%;
    border: 0;
    border-radius: 0;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    background: var(--canvas);
    resize: vertical;
  }
  p {
    color: var(--error-text);
    font-size: 0.75rem;
    margin: 6px 10px;
  }
  .expanded :global(.cm-scroller) {
    max-height: 65vh !important;
    min-height: 45vh !important;
  }
  .compact .toolbar {
    min-height: 24px;
    padding: 2px 6px;
    border: 0;
  }
  .compact .language {
    font-size: 0.6rem;
    padding: 1px 4px;
  }
  .compact :global(.cm-content) {
    padding: 6px 0;
  }
  .compact :global(.cm-line) {
    padding: 0 8px;
  }
  @media (pointer: coarse) {
    .toolbar button {
      min-height: 44px;
      min-width: 44px;
    }
  }
</style>
