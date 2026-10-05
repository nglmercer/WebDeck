<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  import { iconKey, matchingIcons } from '../../lib/icons';
  import IconPickerModal from './IconPickerModal.svelte';

  let { value, onSelect }: { value: string; onSelect: (value: string) => void } = $props();
  let open = $state(false);
  let fullPicker = $state(false);
  let search = $state('');
  let active = $state('');
  let trigger: HTMLButtonElement | undefined;
  let filtered = $derived(matchingIcons(search).slice(0, 12));
  function closePicker() {
    fullPicker = false;
    requestAnimationFrame(() => trigger?.focus());
  }
  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      open = false;
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      if (!filtered.length) return;
      event.preventDefault();
      const index = Math.max(0, filtered.indexOf(active));
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      active = filtered[(index + delta + filtered.length) % filtered.length]!;
      document.getElementById(`quick-icon-${active}`)?.focus();
    }
    if (event.key === 'Enter' && open && active) {
      event.preventDefault();
      onSelect(`icon:${active}`);
      open = false;
    }
  }
</script>

<div class="icon-field" role="group" aria-label={t('ui_icon')}>
  <span class="label" id="icon-field-label">{t('ui_icon')}</span>
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    aria-label={t('ui_icon')}
    aria-labelledby="icon-field-label"
    aria-expanded={open}
    aria-haspopup="listbox"
    onclick={() => {
      open = !open;
      active = filtered[0] ?? '';
    }}
  >
    {#if value.startsWith('icon:')}<Icon
        name={iconKey(value)}
      />{:else if value.startsWith('asset:')}<Icon name="image" />{:else}<Icon name={value} />{/if}
    <span>{value.replace(/^(icon:|asset:)/, '') || t('ui_select_icon')}</span><Icon name="next" />
  </button>
  {#if open}
    <div class="popover">
      <label class="search"
        >{t('ui_search_icons')}<input
          type="search"
          bind:value={search}
          oninput={() => (active = filtered[0] ?? '')}
          onkeydown={onKeydown}
        /></label
      >
      <div
        class="quick"
        role="listbox"
        aria-label={t('ui_search_icons')}
        tabindex="0"
        onkeydown={onKeydown}
      >
        {#each filtered as icon}<button
            id={`quick-icon-${icon}`}
            type="button"
            role="option"
            aria-label={icon}
            aria-selected={iconKey(value) === icon}
            tabindex={icon === active ? 0 : -1}
            onclick={() => {
              onSelect(`icon:${icon}`);
              open = false;
            }}><Icon name={icon} /><span>{icon}</span></button
          >{/each}
      </div>
      <button
        type="button"
        class="browse"
        onclick={() => {
          open = false;
          fullPicker = true;
        }}>{t('ui_browse_all_icons')} <span aria-hidden="true">›</span></button
      >
    </div>
  {/if}
  {#if fullPicker}<IconPickerModal
      value={iconKey(value)}
      close={closePicker}
      select={(name) => onSelect(`icon:${name}`)}
    />{/if}
</div>

<style>
  .icon-field {
    position: relative;
    min-width: 0;
  }
  .label {
    display: block;
    margin-bottom: 6px;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    text-align: left;
  }
  .trigger span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .popover {
    position: absolute;
    z-index: 4;
    top: calc(100% + 5px);
    left: 0;
    width: min(330px, 86vw);
    padding: 10px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
  }
  .search {
    display: block;
    font-size: 0.85rem;
  }
  .search input {
    display: block;
    width: 100%;
    margin-top: 5px;
  }
  .quick {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 5px;
    margin: 9px 0;
    max-height: 180px;
    overflow: auto;
  }
  .quick button {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 7px 3px;
  }
  .quick span {
    font-size: 0.68rem;
    overflow: hidden;
    max-width: 100%;
    text-overflow: ellipsis;
  }
  .quick [aria-selected='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .browse {
    width: 100%;
    display: flex;
    justify-content: space-between;
  }
</style>
