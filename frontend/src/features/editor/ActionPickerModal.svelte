<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { actionOptions, categories, type ActionOption } from './action-options';
  const t = useTranslations();
  let {
    value,
    close,
    select,
  }: { value: string; close: () => void; select: (option: ActionOption) => void } = $props();
  let query = $state('');
  let category = $state('all');
  let selected = $state(untrack(() => value));
  const visible = $derived(
    actionOptions.filter(
      (option) =>
        (category === 'all' || option.category === category) &&
        `${t(option.label)} ${option.command ?? option.type}`
          .toLowerCase()
          .includes(query.trim().toLowerCase()),
    ),
  );
  const candidate = $derived(visible.find((option) => option.id === selected));
</script>

<Modal label={t('ui_select_action')} {close} width="560px">
  <header>
    <h2>{t('ui_select_action')}</h2>
    <button type="button" aria-label={t('ui_close_editor')} onclick={close}
      ><Icon name="close" /></button
    >
  </header>
  <div class="filters">
    <label
      ><span class="sr-only">{t('ui_find_an_action')}</span><input
        type="search"
        bind:value={query}
        placeholder={t('ui_search_actions')}
      /></label
    >
    <label
      ><span class="sr-only">{t('ui_category')}</span><select bind:value={category}
        ><option value="all">{t('ui_all_actions')}</option
        >{#each Object.entries(categories) as [id, label]}<option value={id}>{t(label)}</option
          >{/each}</select
      ></label
    >
  </div>
  <div class="options" role="group" aria-label={t('ui_select_action')}>
    {#each visible as option}<button
        type="button"
        class="option"
        data-action-id={option.id}
        aria-pressed={selected === option.id}
        onclick={() => (selected = option.id)}
        ><Icon name={option.icon} /><span
          ><strong>{t(option.label)}</strong>{#if option.description}<small
              >{t(option.description)}</small
            >{/if}</span
        ></button
      >{/each}
    {#if !visible.length}<p role="status">{t('ui_no_actions_found')}</p>{/if}
  </div>
  <footer>
    <button type="button" onclick={close}>{t('ui_cancel')}</button><button
      type="button"
      class="primary"
      disabled={!candidate}
      onclick={() => {
        if (candidate) {
          select(candidate);
          close();
        }
      }}>{t('ui_select_action')}</button
    >
  </footer>
</Modal>

<style>
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
  }
  .filters {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 150px;
    gap: 8px;
  }
  label {
    margin: 0;
  }
  .options {
    display: grid;
    gap: 5px;
    max-height: 48dvh;
    overflow: auto;
    padding: 4px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    text-align: left;
    padding: 12px;
  }
  .option span {
    min-width: 0;
  }
  strong {
    font-size: 0.9rem;
    font-weight: 500;
  }
  small {
    display: block;
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.75rem;
    line-height: 1.4;
  }
  .option[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  footer {
    justify-content: flex-end;
    padding-top: 10px;
  }
  @media (max-width: 480px) {
    .filters {
      grid-template-columns: 1fr;
    }
  }
</style>
