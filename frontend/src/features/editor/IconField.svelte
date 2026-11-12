<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { iconKey } from '../../lib/icons';
  import IconPickerModal from './IconPickerModal.svelte';
  const t = useTranslations();
  let { value, onSelect }: { value: string; onSelect: (value: string) => void } = $props();
  let open = $state(false);
</script>

<div class="icon-field">
  <span class="label">{t('editor.icon')}</span>
  <button
    type="button"
    class="trigger"
    aria-label={t('editor.icon')}
    aria-haspopup="dialog"
    data-icon={value}
    onclick={() => (open = true)}
  >
    <Icon name={value.startsWith('asset:') ? 'image' : iconKey(value)} />
    <span
      >{value.startsWith('asset:')
        ? t('images.customImages')
        : iconKey(value) || t('icons.selectIcon')}</span
    >
    <Icon name="next" />
  </button>
  {#if open}<IconPickerModal
      value={iconKey(value)}
      close={() => (open = false)}
      select={(name) => onSelect(`icon:${name}`)}
    />{/if}
</div>

<style>
  .icon-field {
    min-width: 0;
  }
  .label {
    display: block;
    margin-bottom: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 42px;
    text-align: left;
  }
  .trigger span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
