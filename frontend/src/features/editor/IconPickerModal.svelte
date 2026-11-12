<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Modal from '../../components/Modal.svelte';
  import { iconCategory, matchingIcons } from '../../lib/icons';
  import { useTranslations, common, keys } from '../../lib/i18n';
  const t = useTranslations();

  let {
    value,
    close,
    select,
  }: { value: string; close: () => void; select: (icon: string) => void } = $props();
  let query = $state('');
  let category = $state('all');
  let highlighted = $state('');
  const categories = {
    all: keys.icons.categories.all,
    media: common.media,
    devices: common.devices,
    ui: keys.icons.categories.ui,
    folders: keys.icons.categories.folders,
    system: keys.icons.categories.system,
  };
  let icons = $derived(
    matchingIcons(query).filter((name) => category === 'all' || iconCategory(name) === category),
  );
  $effect(() => {
    if (!icons.includes(highlighted))
      highlighted = icons.includes(value) ? value : (icons[0] ?? '');
  });
  let grid: HTMLDivElement;
  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (icons.includes(highlighted)) {
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
      const columns = getComputedStyle(grid).gridTemplateColumns.split(' ').length;
      const delta =
        event.key === 'ArrowDown'
          ? columns
          : event.key === 'ArrowUp'
            ? -columns
            : event.key === 'ArrowRight'
              ? 1
              : -1;
      highlighted = icons[(index + delta + icons.length) % icons.length]!;
      document.getElementById(`full-icon-${highlighted}`)?.focus();
    }
  }
</script>

<Modal label={t('icons.selectIcon')} {close} width="680px">
  <div class="picker">
    <header>
      <h2>{t('icons.selectIcon')}</h2>
      <button type="button" aria-label={t('editor.closeEditor')} onclick={close}
        ><Icon name="close" /></button
      >
    </header>
    <div class="tools">
      <nav aria-label={t('icons.category')}>
        {#each Object.entries(categories) as [item, label]}<button
            type="button"
            aria-pressed={category === item}
            onclick={() => {
              category = item;
            }}>{t(label)}</button
          >{/each}
      </nav>
      <label>{t('icons.searchIcons')}<input type="search" bind:value={query} /></label>
    </div>
    <div
      bind:this={grid}
      class="icons"
      role="listbox"
      aria-label={t('icons.selectIcon')}
      tabindex={icons.length ? -1 : 0}
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
      {#if !icons.length}<p class="empty" role="status">{t('icons.noMatchingIcons')}</p>{/if}
    </div>
    <footer>
      <button type="button" onclick={close}>{t(common.cancel)}</button><button
        type="button"
        class="primary"
        disabled={!icons.includes(highlighted)}
        onclick={() => {
          select(highlighted);
          close();
        }}>{t(common.select)}</button
      >
    </footer>
  </div>
</Modal>

<style>
  .picker {
    width: 100%;
    min-width: 0;
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
    grid-template-columns: 1fr;
    gap: 12px;
    margin: 14px 0;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  nav button {
    text-align: left;
    padding: 8px 12px;
  }
  nav [aria-pressed='true'] {
    background: var(--accent-surface);
  }
  .icons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(76px, 1fr));
    gap: 8px;
    max-height: 45dvh;
    min-height: 120px;
    align-content: start;
    padding: 4px;
    overflow: auto;
  }
  .icons button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    min-height: 80px;
    min-width: 0;
    padding: 12px 6px;
  }
  .icons [aria-selected='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .icons span {
    font-size: 0.72rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tools label {
    margin: 0;
  }
  .empty {
    grid-column: 1 / -1;
    text-align: center;
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
      flex-wrap: wrap;
    }
  }
</style>
