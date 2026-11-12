<script lang="ts">
  import Icon from '../../../components/Icon.svelte';
  import { useTranslations } from '../../../lib/i18n';
  const t = useTranslations();
  let {
    label,
    accept,
    onchange,
  }: { label: string; accept: string; onchange: (event: Event) => void } = $props();
  let filename = $state('');
</script>

<label class="file-field"
  ><Icon name="image" size={24} /><span class="file-copy"
    ><strong>{label}</strong><small>{filename || t('ui_choose_file')}</small></span
  ><span class="file-action" aria-hidden="true">{t('ui_choose_file')}</span>
  <input
    class="sr-only"
    type="file"
    aria-label={label}
    {accept}
    onchange={(event) => {
      filename = event.currentTarget.files?.[0]?.name ?? '';
      onchange(event);
    }}
  />
</label>

<style>
  .file-field {
    position: relative;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 12px;
    margin: 12px 0;
    padding: 16px;
    min-height: 76px;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    background: var(--canvas);
  }
  .file-field:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .file-field:hover {
    border-color: var(--border);
  }
  .file-copy {
    min-width: 0;
    flex: 1;
  }
  strong {
    display: block;
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 500;
  }
  small {
    display: block;
    overflow-wrap: anywhere;
    margin-top: 5px;
    font-size: 0.75rem;
  }
  .file-action {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    color: var(--text);
    font-size: 0.75rem;
  }
  @media (max-width: 480px) {
    .file-action {
      display: none;
    }
  }
</style>
