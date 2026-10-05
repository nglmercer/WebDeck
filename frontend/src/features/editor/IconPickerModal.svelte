<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Modal from '../../components/Modal.svelte';
  import { iconCategory, matchingIcons } from '../../lib/icons';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  let {
    value,
    close,
    select,
  }: { value: string; close: () => void; select: (icon: string) => void } = $props();
  let query = $state('');
  let category = $state('all');
  let highlighted = $state('');
  $effect(() => {
    if (!highlighted)
      highlighted = matchingIcons('').includes(value) ? value : (matchingIcons('')[0] ?? '');
  });
  const categories = ['all', 'media', 'devices', 'ui', 'folders', 'system'];
  let icons = $derived(
    matchingIcons(query).filter((name) => category === 'all' || iconCategory(name) === category),
  );
  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (highlighted) {
        select(highlighted);
        close();
      }
    } else if (
      event.key === 'ArrowDown' ||
      event.key === 'ArrowRight' ||
      event.key === 'ArrowUp' ||
      event.key === 'ArrowLeft'
    ) {
      if (!icons.length) return;
      event.preventDefault();
      const index = Math.max(0, icons.indexOf(highlighted));
      const delta = event.key === 'ArrowDown' || event.key === 'ArrowRight' ? 1 : -1;
      highlighted = icons[(index + delta + icons.length) % icons.length]!;
      document.getElementById(`full-icon-${highlighted}`)?.focus();
    }
  }
</script>

<Modal label={t('ui_select_icon')} {close}>
  <div class="picker">
    <header>
      <h2>{t('ui_select_icon')}</h2>
      <button type="button" aria-label={t('ui_close_editor')} onclick={close}
        ><Icon name="close" /></button
      >
    </header>
    <div class="tools">
      <nav aria-label={t('ui_icon_categories')}>
        {#each categories as item}<button
            type="button"
            aria-pressed={category === item}
            onclick={() => {
              category = item;
              highlighted = icons[0] ?? '';
            }}>{t(`ui_icon_category_${item}`)}</button
          >{/each}
      </nav>
      <label
        >{t('ui_search_icons')}<input
          type="search"
          bind:value={query}
          oninput={() => (highlighted = icons[0] ?? '')}
          onkeydown={onKeydown}
        /></label
      >
    </div>
    <div
      class="icons"
      role="listbox"
      aria-label={t('ui_select_icon')}
      tabindex="0"
      onkeydown={onKeydown}
    >
      {#each icons as icon}<button
          id={`full-icon-${icon}`}
          type="button"
          role="option"
          aria-label={icon}
          aria-selected={highlighted === icon}
          tabindex={highlighted === icon ? 0 : -1}
          onclick={() => (highlighted = icon)}
          ondblclick={() => {
            select(icon);
            close();
          }}><Icon name={icon} /><span>{icon}</span></button
        >{/each}
    </div>
    <footer>
      <button type="button" onclick={close}>{t('ui_cancel')}</button><button
        type="button"
        class="primary"
        disabled={!highlighted}
        onclick={() => {
          select(highlighted);
          close();
        }}>{t('ui_select')}</button
      >
    </footer>
  </div>
</Modal>

<style>
  .picker {
    min-width: min(620px, 76vw);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .tools {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 12px;
    margin: 14px 0;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  nav button {
    text-align: left;
  }
  nav [aria-pressed='true'] {
    background: var(--accent-surface);
  }
  .icons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(76px, 1fr));
    gap: 8px;
    max-height: 45dvh;
    overflow: auto;
  }
  .icons button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    min-height: 68px;
  }
  .icons [aria-selected='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .icons span {
    font-size: 0.72rem;
  }
  footer {
    justify-content: flex-end;
    margin-top: 14px;
  }
  @media (max-width: 560px) {
    .picker {
      min-width: 0;
    }
    .tools {
      grid-template-columns: 1fr;
    }
    nav {
      flex-direction: row;
      overflow: auto;
    }
  }
</style>
