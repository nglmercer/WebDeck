<script lang="ts">
  import { untrack, tick } from 'svelte';
  import Icon from '../../components/Icon.svelte';
  import JsonField from './JsonField.svelte';
  import { id } from '../../lib/id';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    label,
    value,
    onchange,
    stringValues = false,
    headers = false,
  }: {
    label: string;
    value: Record<string, unknown>;
    onchange: (value: Record<string, unknown>) => void;
    stringValues?: boolean;
    headers?: boolean;
  } = $props();
  type Row = { id: string; key: string; value: unknown };
  const makeRows = (value: Record<string, unknown>) =>
    Object.entries(value).map(([key, value]) => ({ id: id(), key, value }));
  let rows = $state<Row[]>(untrack(() => makeRows(value)));
  let accepted = untrack(() => JSON.stringify(value));
  let root: HTMLElement;
  const errors = $derived(
    rows.map((row) =>
      !row.key.trim()
        ? t('forms.map.emptyKey')
        : rows.filter((other) =>
              headers ? other.key.toLowerCase() === row.key.toLowerCase() : other.key === row.key,
            ).length > 1
          ? t('forms.map.duplicateKey')
          : headers &&
              (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(row.key) || /[\r\n]/.test(String(row.value)))
            ? t('forms.http.invalidHeader')
            : '',
    ),
  );
  $effect(() => {
    const next = JSON.stringify(value);
    if (next !== accepted) {
      accepted = next;
      rows = makeRows(value);
    }
  });
  function commit() {
    const result = Object.fromEntries(rows.map((row) => [row.key, row.value]));
    accepted = JSON.stringify(result);
    onchange(result);
  }
  function validate(input: HTMLInputElement, message: string) {
    input.setCustomValidity(message);
    return {
      update(message: string) {
        input.setCustomValidity(message);
      },
    };
  }
  async function add() {
    const row = { id: id(), key: '', value: '' };
    rows = [...rows, row];
    commit();
    await tick();
    root.querySelector<HTMLInputElement>(`[data-key-id="${row.id}"]`)?.focus();
  }
  async function remove(index: number) {
    rows = rows.filter((_, i) => i !== index);
    commit();
    await tick();
    const inputs = root.querySelectorAll<HTMLInputElement>('[data-key-id]');
    (
      inputs[Math.min(index, inputs.length - 1)] ?? root.querySelector<HTMLButtonElement>('.add')
    )?.focus();
  }
</script>

<section class="map-field" bind:this={root} aria-label={label}>
  <div class="map-heading">
    <h3>{label}</h3>
    <button type="button" class="add text-button" onclick={add}
      ><Icon name="plus" size={17} />{t(
        headers ? 'forms.http.addHeader' : 'editor.addField',
      )}</button
    >
  </div>
  {#if rows.length}<div class="column-labels" aria-hidden="true">
      <span>{t('forms.map.key')}</span><span>{t('forms.map.value')}</span>
    </div>{/if}
  {#each rows as row, index (row.id)}<div class="map-row">
      <input
        data-key-id={row.id}
        aria-label={t('forms.map.keyAt', { index: index + 1 })}
        placeholder={t(headers ? 'forms.http.headerName' : 'forms.map.key')}
        value={row.key}
        required
        maxlength="256"
        aria-invalid={!!errors[index]}
        aria-describedby={errors[index] ? row.id + '-error' : undefined}
        use:validate={errors[index] ?? ''}
        oninput={(event) => {
          row.key = event.currentTarget.value;
          commit();
        }}
      />
      <div class="value-field">
        {#if stringValues}<input
            aria-label={t('forms.map.valueAt', { index: index + 1 })}
            placeholder={t('forms.map.value')}
            value={String(row.value ?? '')}
            maxlength="8192"
            oninput={(event) => {
              row.value = event.currentTarget.value;
              commit();
            }}
          />{:else}<JsonField
            compact
            hideLabel
            value={row.value}
            label={t('forms.map.valueAt', { index: index + 1 })}
            onchange={(next) => {
              row.value = next;
              commit();
            }}
          />{/if}
      </div>
      <button
        type="button"
        class="remove"
        aria-label={t('forms.map.removeAt', { index: index + 1 })}
        title={t('forms.map.removeAt', { index: index + 1 })}
        onclick={() => remove(index)}><Icon name="close" size={19} /></button
      >
      {#if errors[index]}<p id={row.id + '-error'} class="error" role="alert">
          {errors[index]}
        </p>{/if}
    </div>{/each}
</section>

<style>
  .map-field {
    min-width: 0;
  }
  .value-field {
    min-width: 0;
  }
  .map-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 6px;
  }
  .map-heading h3 {
    margin: 0;
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 550;
  }
  .map-heading button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    min-height: 44px;
    background: transparent;
    border: 0;
    color: var(--accent);
    font-size: 0.8rem;
  }
  .column-labels,
  .map-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.45fr) 44px;
    gap: 8px;
  }
  .column-labels {
    color: var(--text-muted);
    font-size: 0.72rem;
    padding: 0 4px 5px;
  }
  .map-row {
    align-items: start;
    margin-bottom: 8px;
  }
  .map-row input {
    width: 100%;
    min-width: 0;
    min-height: 44px;
    padding: 8px;
    font-size: 0.82rem;
  }
  .remove {
    display: grid;
    place-items: center;
    min-height: 44px;
    width: 44px;
    padding: 8px;
    background: transparent;
    border-color: transparent;
  }
  .remove:hover {
    color: var(--error-text);
    background: var(--error-surface);
  }
  .error {
    grid-column: 1/-1;
    color: var(--error-text);
    font-size: 0.75rem;
    margin: 0;
  }
  @media (max-width: 480px) {
    .column-labels {
      display: none;
    }
    .map-row {
      grid-template-columns: minmax(0, 1fr) 44px;
      padding-bottom: 12px;
      margin-bottom: 12px;
      border-bottom: 1px solid var(--border-subtle);
    }
    .value-field {
      grid-column: 1 / -1;
      grid-row: 2;
    }
    .remove {
      grid-column: 2;
      grid-row: 1;
    }
  }
</style>
