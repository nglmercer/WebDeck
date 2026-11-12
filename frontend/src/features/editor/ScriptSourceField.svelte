<script lang="ts">
  import { untrack } from 'svelte';
  import SegmentedControl from '../../components/SegmentedControl.svelte';
  import CodeEditor from '../../components/CodeEditor.svelte';
  import FileSourceField from './FileSourceField.svelte';
  import { useTranslations } from '../../lib/i18n';
  import type { MediaMetadata } from './audio';
  import type { FileSource, ScriptSource } from '../../lib/contracts';
  const t = useTranslations();
  let {
    value,
    onchange,
    language = 'javascript',
    metadata,
    onmetadata,
  }: {
    value: ScriptSource;
    onchange: (source: ScriptSource) => void;
    language?: string;
    metadata?: MediaMetadata | undefined;
    onmetadata?: ((metadata: MediaMetadata) => void) | undefined;
  } = $props();
  let code = untrack(() => (value.type === 'inline' ? value.code : ''));
  let file: FileSource = untrack(() =>
    value.type === 'file' ? value.source : { type: 'asset', id: '' },
  );
  function mode(next: string) {
    if (value.type === 'inline') code = value.code;
    else file = value.source;
    onchange(next === 'inline' ? { type: 'inline', code } : { type: 'file', source: file });
  }
</script>

<div class="script-source">
  <SegmentedControl
    label={t('forms.source.title')}
    value={value.type}
    options={[
      { value: 'inline', label: t('forms.source.code') },
      { value: 'file', label: t('forms.source.file') },
    ]}
    onchange={mode}
  />
  {#if value.type === 'inline'}<CodeEditor
      label={t('forms.source.code')}
      hideLabel
      value={value.code}
      {language}
      onchange={(code) => onchange({ type: 'inline', code })}
    />{:else}<FileSourceField
      label={t('forms.source.file')}
      hideLabel
      value={value.source}
      accept={language === 'javascript'
        ? '.js,.mjs,.cjs,.txt'
        : '.sh,.bash,.zsh,.bat,.cmd,.ps1,.txt'}
      maxSize={65536}
      {metadata}
      {onmetadata}
      onchange={(source) => onchange({ type: 'file', source })}
    />{/if}
</div>

<style>
  .script-source {
    display: grid;
    gap: 14px;
    min-width: 0;
  }
</style>
