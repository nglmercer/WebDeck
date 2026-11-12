<script lang="ts">
  import { untrack } from 'svelte';
  import FilterSelect from '../../components/FilterSelect.svelte';
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations, common } from '../../lib/i18n';
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
        `${t.message(option.label)} ${option.command ?? option.type}`
          .toLowerCase()
          .includes(query.trim().toLowerCase()),
    ),
  );
  const candidate = $derived(visible.find((option) => option.id === selected));
</script>

<Modal label={t('actions.selectAction')} {close} width="560px">
  <div class="guide-action-picker" data-guide="action-picker">
    <header>
      <h2>{t('actions.selectAction')}</h2>
      <button type="button" aria-label={t('editor.closeEditor')} onclick={close}
        ><Icon name="close" /></button
      >
    </header>
    <div class="filters" data-guide="action-picker-filters">
      <label
        ><span class="sr-only">{t('actions.findAnAction')}</span><input
          type="search"
          bind:value={query}
          placeholder={t('actions.searchActions')}
        /></label
      >
      <FilterSelect
        label={t('actions.category')}
        value={category}
        hideLabel
        options={[
          { value: 'all', label: t('actions.allActions') },
          ...Object.entries(categories).map(([value, label]) => ({ value, label: t(label) })),
        ]}
        onchange={(value) => (category = value)}
      />
    </div>
    <div class="options" role="group" aria-label={t('actions.selectAction')}>
      {#each visible as option}<button
          type="button"
          class="option"
          data-action-id={option.id}
          aria-pressed={selected === option.id}
          onclick={() => (selected = option.id)}
          ><Icon name={option.icon} /><span
            ><strong>{t.message(option.label)}</strong>{#if option.description}<small
                >{t.message(option.description)}</small
              >{/if}</span
          ></button
        >{/each}
      {#if !visible.length}<p role="status">{t('actions.noActionsFound')}</p>{/if}
    </div>
    <footer>
      <button type="button" onclick={close}>{t(common.cancel)}</button><button
        type="button"
        class="primary"
        disabled={!candidate}
        onclick={() => {
          if (candidate) {
            select(candidate);
            close();
          }
        }}>{t('actions.selectAction')}</button
      >
    </footer>
  </div>
</Modal>

<style>
  .guide-action-picker {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
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
