<script lang="ts">
  import { untrack } from 'svelte';
  import CodeEditor from '../../components/CodeEditor.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    value,
    onchange,
    label,
    compact = false,
    hideLabel = false,
  }: {
    value: unknown;
    onchange: (value: unknown) => void;
    label: string;
    compact?: boolean;
    hideLabel?: boolean;
  } = $props();
  let text = $state(untrack(() => JSON.stringify(value, null, 2) ?? ''));
  let accepted = untrack(() => JSON.stringify(value));
  let error = $state('');
  $effect(() => {
    const next = JSON.stringify(value);
    if (next !== accepted) {
      accepted = next;
      text = JSON.stringify(value, null, 2) ?? '';
      error = '';
    }
  });
  function change(next: string) {
    text = next;
    try {
      const parsed: unknown = JSON.parse(next);
      accepted = JSON.stringify(parsed);
      error = '';
      onchange(parsed);
    } catch {
      error = t('errors.invalidJson');
    }
  }
</script>

<CodeEditor {label} value={text} onchange={change} language="json" {error} {compact} {hideLabel} />
