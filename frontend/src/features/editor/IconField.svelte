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
  <span class="label">{t('ui_icon')}</span>
  <button
    type="button"
    class="trigger"
    aria-label={t('ui_icon')}
    aria-haspopup="dialog"
    onclick={() => (open = true)}
  >
    <Icon name={value.startsWith('asset:') ? 'image' : iconKey(value)} />
    <span
      >{value.startsWith('asset:')
        ? t('ui_custom_images')
        : iconKey(value) || t('ui_select_icon')}</span
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
