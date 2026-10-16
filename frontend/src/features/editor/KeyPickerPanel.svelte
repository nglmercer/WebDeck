<script lang="ts">
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
  let {
    candidate,
    onCandidateChange,
    advanced,
    onAdvancedChange,
    capabilities,
  }: {
    candidate: string[];
    onCandidateChange: (keys: string[]) => void;
    advanced: boolean;
    onAdvancedChange: (advanced: boolean) => void;
    capabilities: KeyboardCapabilities | undefined;
  } = $props();

  let category = $state('function');
  let search = $state('');
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
      const matchesCategory = search.trim() || key.category === category;
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
  <section class="selection" aria-labelledby="picker-selection-title">
    <h3 id="picker-selection-title">{t('ui_keypicker_shortcut')}</h3>
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
    {:else}
      <p class="empty-selection">{t('ui_keypicker_selection_empty')}</p>
    {/if}
  </section>

  <fieldset class="modifiers">
    <legend>{t('ui_keypicker_modifiers')}</legend>
    <div class="modifier-buttons">
      {#each modifiers as modifier (modifier)}
        <button
          type="button"
          aria-pressed={selected(modifier)}
          onclick={() => onCandidateChange(toggleModifier(candidate, modifier))}
          >{keyLabel(modifier)}</button
        >
      {/each}
    </div>
  </fieldset>

  <div class="advanced">
    <label>
      <input
        type="checkbox"
        checked={advanced}
        onchange={(event) => onAdvancedChange(event.currentTarget.checked)}
      />
      {t('ui_keypicker_advanced_simultaneous')}
    </label>
    {#if advanced}<p>{t('ui_keypicker_advanced_help')}</p>{/if}
  </div>

  <label class="search">
    {t('ui_keypicker_search_keys')}
    <input type="search" bind:value={search} autocomplete="off" />
  </label>

  {#if !search.trim()}
    <div class="categories" role="group" aria-label={t('ui_keypicker_category')}>
      {#each categories as group (group)}
        <button type="button" aria-pressed={category === group} onclick={() => (category = group)}
          >{t(categoryMessages[group]!)}</button
        >
      {/each}
    </div>
  {/if}

  <div class="host-status" role="status">{t(keyStatusMessage('f1', capabilities))}</div>
  {#if visible.length}
    <div class="key-grid" role="group" aria-label={t('ui_keypicker_choose_primary_key')}>
      {#each visible as key (key.id)}
        {@const unavailable = keyUnavailable(key.id, capabilities)}
        {@const support = platformSupport(key.id, capabilities?.platform)}
        <button
          type="button"
          aria-pressed={selected(key.id)}
          aria-label={`${key.label}, ${t(keyStatusMessage(key.id, capabilities))}`}
          aria-describedby={unavailable ? 'key-picker-unavailable-help' : undefined}
          disabled={unavailable}
          data-support={support}
          onkeydown={moveGridFocus}
          onclick={() => onCandidateChange(selectPrimaryKey(candidate, key.id, advanced))}
          >{key.label}</button
        >
      {/each}
    </div>
  {:else}
    <p class="no-results" role="status">{t('ui_keypicker_no_results')}</p>
  {/if}

  {#if hasUnavailableVisible}
    <p id="key-picker-unavailable-help" class="help">{t('ui_keypicker_unavailable_help')}</p>
  {/if}

  <button type="button" class="clear" onclick={() => onCandidateChange([])}>
    {t('ui_keypicker_clear_selection')}
  </button>
  {#if !candidate.length || !hasPrimaryKey}
    <p class="help">{t('ui_keypicker_empty_help')}</p>
  {:else if validationMessage}
    <p class="help" role="alert">{t(validationMessage)}</p>
  {/if}
</div>

<style>
  .picker-fields {
    display: grid;
    gap: 12px;
    min-width: 0;
  }
  h3,
  p {
    margin: 0;
  }
  h3 {
    font-size: 0.9rem;
  }
  .selection {
    display: grid;
    gap: 6px;
    max-width: none;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .chips,
  .modifier-buttons,
  .categories {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip,
  .plus {
    min-height: 36px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
  }
  .empty-selection {
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  fieldset {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }
  legend {
    margin-bottom: 6px;
    font-size: 0.82rem;
    font-weight: 600;
  }
  button {
    min-height: 44px;
    padding: 7px 10px;
  }
  [aria-pressed='true'] {
    outline: 2px solid var(--accent, currentColor);
    background: var(--accent-surface, #ffffff18);
  }
  .advanced {
    padding: 8px 10px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
  }
  .advanced label {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    margin: 0;
  }
  .advanced input {
    width: 18px;
    height: 18px;
    margin: 0;
  }
  .advanced p,
  .help,
  .host-status,
  .no-results {
    color: var(--text-muted);
    font-size: 0.78rem;
    line-height: 1.4;
  }
  .advanced p {
    margin: 6px 0 0 26px;
  }
  .search {
    display: grid;
    gap: 5px;
    margin: 0;
  }
  .search input {
    min-height: 42px;
  }
  .categories {
    gap: 5px;
  }
  .categories button {
    min-height: 38px;
    padding: 5px 9px;
    font-size: 0.76rem;
  }
  .key-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(52px, 1fr));
    gap: 6px;
    max-height: 190px;
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
  }
  .key-grid button:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }
  .clear {
    justify-self: start;
  }
  @media (max-width: 640px) {
    .key-grid {
      max-height: min(30dvh, 220px);
    }
    .categories button {
      min-height: 44px;
    }
  }
</style>
