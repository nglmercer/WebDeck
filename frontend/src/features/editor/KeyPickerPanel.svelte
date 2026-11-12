<script lang="ts">
  import FilterSelect from '../../components/FilterSelect.svelte';
  import { id } from '../../lib/id';
  import { useTranslations } from '../../lib/i18n';
  import type { KeyboardCapabilities } from '../../lib/contracts';
  import {
    keyLabel,
    keyStatusMessage,
    keyUnavailable,
    keys,
    modifiers,
    normalizeKey,
    platformSupport,
    selectPrimaryKey,
    toggleModifier,
    validatePickerChordForHost,
  } from './keys';

  const t = useTranslations();
  const selectionId = `selection-${id()}`;
  let {
    candidate,
    guideStep,
    onCandidateChange,
    advanced,
    onAdvancedChange,
    capabilities,
  }: {
    candidate: string[];
    guideStep?: string | undefined;
    onCandidateChange: (keys: string[]) => void;
    advanced: boolean;
    onAdvancedChange: (advanced: boolean) => void;
    capabilities: KeyboardCapabilities | undefined;
  } = $props();

  let category = $state('function');
  let search = $state('');
  $effect(() => {
    if (guideStep === 'keys') {
      search = '';
      category = 'function';
    }
  });
  const categoryMessages: Record<string, string> = {
    function: 'ui_keypicker_function_keys',
    characters: 'ui_keypicker_letters_numbers',
    navigation: 'ui_keypicker_navigation',
    system: 'ui_keypicker_editing_special',
    numpad: 'ui_keypicker_numpad',
    media: 'ui_keypicker_media_system',
    hardware: 'ui_keypicker_hardware',
  };
  const categories = [
    ...new Set(keys.map((key) => key.category).filter((group) => group !== 'modifiers')),
  ];
  const visible = $derived(
    keys.filter((key) => {
      if (key.category === 'modifiers') return false;
      const matchesCategory = category === 'all' || key.category === category;
      const query = search.trim().toLowerCase();
      const matchesSearch =
        !query ||
        `${key.label} ${key.id} ${key.category} ${key.aliases.join(' ')}`
          .toLowerCase()
          .includes(query);
      return matchesCategory && matchesSearch;
    }),
  );
  const hasUnavailableVisible = $derived(
    visible.some((key) => keyUnavailable(key.id, capabilities)),
  );
  const validationMessage = $derived(validatePickerChordForHost(candidate, advanced, capabilities));
  const hasPrimaryKey = $derived(
    candidate.some((key) => !modifiers.includes(normalizeKey(key) as (typeof modifiers)[number])),
  );

  function selected(id: string): boolean {
    return candidate.some((key) => normalizeKey(key) === id);
  }

  function removeAt(index: number) {
    onCandidateChange(candidate.filter((_, itemIndex) => itemIndex !== index));
  }

  function moveGridFocus(event: KeyboardEvent) {
    if (!(event.currentTarget instanceof HTMLButtonElement)) return;
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) return;
    const grid = event.currentTarget.closest('.key-grid');
    if (!(grid instanceof HTMLDivElement)) return;
    const buttons = [...grid.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    const index = buttons.indexOf(event.currentTarget);
    if (index < 0) return;
    const columns = Math.max(
      1,
      window.getComputedStyle(grid).gridTemplateColumns.split(' ').filter(Boolean).length,
    );
    const delta =
      event.key === 'ArrowLeft'
        ? -1
        : event.key === 'ArrowRight'
          ? 1
          : event.key === 'ArrowUp'
            ? -columns
            : columns;
    const next = Math.max(0, Math.min(buttons.length - 1, index + delta));
    if (next !== index) {
      event.preventDefault();
      buttons[next]?.focus();
    }
  }
</script>

<div class="picker-fields">
  <section class="selection" aria-labelledby={selectionId}>
    <div class="selection-heading">
      <h3 id={selectionId}>{t('ui_keypicker_shortcut')}</h3>
      <button
        type="button"
        class="clear"
        disabled={!candidate.length}
        onclick={() => onCandidateChange([])}>{t('ui_keypicker_clear_selection')}</button
      >
    </div>
    {#if candidate.length}
      <div class="chips" aria-label={t('ui_keypicker_selected_keys')}>
        {#each candidate as key, index (`${key}-${index}`)}
          <button
            type="button"
            class="chip"
            aria-label={t('ui_keypicker_remove', { key: keyLabel(key) })}
            onclick={() => removeAt(index)}
            >{keyLabel(key)} <span aria-hidden="true">×</span></button
          >
          {#if index < candidate.length - 1}<span class="plus" aria-hidden="true">+</span>{/if}
        {/each}
      </div>
    {:else}<p class="empty-selection">{t('ui_keypicker_choose_one')}</p>{/if}
  </section>

  <fieldset class="modifiers">
    <legend>{t('ui_keypicker_modifiers')}</legend>
    <div class="modifier-buttons">
      {#each modifiers as modifier (modifier)}<button
          type="button"
          aria-pressed={selected(modifier)}
          onclick={() => onCandidateChange(toggleModifier(candidate, modifier))}
          >{keyLabel(modifier)}</button
        >{/each}
    </div>
  </fieldset>

  <div class="filters">
    <label class="search"
      ><span class="sr-only">{t('ui_keypicker_search_keys')}</span>
      <input
        type="search"
        value={search}
        placeholder={t('ui_keypicker_search_keys')}
        autocomplete="off"
        oninput={(event) => {
          search = event.currentTarget.value;
          if (search.trim()) category = 'all';
        }}
      />
    </label>
    <FilterSelect
      label={t('ui_keypicker_category')}
      value={category}
      hideLabel
      options={[
        { value: 'all', label: t('ui_keypicker_all_keys') },
        ...categories.map((group) => ({ value: group, label: t(categoryMessages[group]!) })),
      ]}
      onchange={(value) => (category = value)}
    />
  </div>

  {#if visible.length}
    <div class="key-grid" role="group" aria-label={t('ui_keypicker_choose_primary_key')}>
      {#each visible as key (key.id)}
        {@const unavailable = keyUnavailable(key.id, capabilities)}
        {@const support = platformSupport(key.id, capabilities?.platform)}
        {@const label = key.id === 'fn' ? 'Fn' : key.label}
        <button
          type="button"
          aria-pressed={selected(key.id)}
          aria-label={label}
          title={unavailable ? t(keyStatusMessage(key.id, capabilities)) : undefined}
          aria-describedby={unavailable ? `${selectionId}-unavailable` : undefined}
          disabled={unavailable}
          data-support={support}
          onkeydown={moveGridFocus}
          onclick={() => onCandidateChange(selectPrimaryKey(candidate, key.id, advanced))}
          >{label}</button
        >
      {/each}
    </div>
  {:else}<p class="no-results" role="status">{t('ui_keypicker_no_results')}</p>{/if}

  <div class="advanced">
    <label title={t('ui_keypicker_advanced_help')}
      ><input
        type="checkbox"
        checked={advanced}
        onchange={(event) => onAdvancedChange(event.currentTarget.checked)}
      />{t('ui_keypicker_multiple_keys')}</label
    >
    {#if advanced}<p>{t('ui_keypicker_simultaneous_hint')}</p>{/if}
  </div>
  {#if hasUnavailableVisible}<p id={`${selectionId}-unavailable`} class="help">
      {t('ui_keypicker_unavailable_hint')}
    </p>{/if}
  {#if candidate.length && !hasPrimaryKey}<p class="help" role="status">
      {t('ui_keypicker_select_primary')}
    </p>
  {:else if candidate.length && validationMessage}<p class="help" role="alert">
      {t(validationMessage)}
    </p>{/if}
</div>

<style>
  .picker-fields {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  h3,
  p {
    margin: 0;
  }
  h3 {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-muted);
  }
  .selection {
    display: grid;
    gap: 6px;
    max-width: none;
    margin: 0;
    padding: 0 0 10px;
    border: 0;
    border-bottom: 1px solid var(--border-subtle);
    border-radius: 0;
    background: transparent;
  }
  .selection-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-height: 32px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--canvas);
    font-size: 0.85rem;
  }
  .chip span,
  .plus {
    color: var(--text-muted);
  }
  .empty-selection {
    min-height: 32px;
    display: flex;
    align-items: center;
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .clear {
    min-height: 24px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.72rem;
  }
  .clear:hover:not(:disabled) {
    color: var(--text);
  }
  fieldset {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }
  legend {
    margin-bottom: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .modifier-buttons {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }
  .modifier-buttons button {
    min-height: 40px;
    padding: 7px;
    font-size: 0.85rem;
  }
  [aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .advanced {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
  }
  .advanced label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 0.8rem;
  }
  .advanced input {
    width: 16px;
    height: 16px;
    min-height: 16px;
    margin: 0;
    accent-color: var(--accent);
  }
  .advanced p,
  .help,
  .no-results {
    color: var(--text-muted);
    font-size: 0.75rem;
    line-height: 1.4;
  }
  .filters {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(140px, 0.85fr);
    gap: 8px;
  }
  .search {
    margin: 0;
  }
  .search input {
    min-height: 44px;
    border-radius: var(--radius-control);
    font-size: 0.8rem;
  }
  .key-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
    gap: 6px;
    max-height: min(30dvh, 208px);
    overflow: auto;
    overscroll-behavior: contain;
    padding: 2px;
  }
  .key-grid button {
    display: grid;
    place-items: center;
    min-width: 0;
    min-height: 44px;
    padding: 4px;
    font-size: 0.8rem;
  }
  .key-grid button:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }
  @media (max-width: 380px) {
    .filters {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
