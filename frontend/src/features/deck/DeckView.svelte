<script lang="ts">
  import { onMount } from 'svelte';
  import { onRuntimeEvent } from '../../lib/api/realtime';
  let dynamic = $state<
    Record<string, { label?: string | undefined; active?: boolean | undefined }>
  >({});
  onMount(() =>
    onRuntimeEvent((event) => {
      if (event.type === 'runtime.reloaded') dynamic = {};
      else if (Object.keys(dynamic).length < 4096 || event.button_id in dynamic)
        dynamic[event.button_id] = { label: event.label, active: event.active };
    }),
  );
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Button, Layout, UsageResponse } from '../../lib/contracts';
  import { appearanceNumber, gridCells, gridWindow, placeButtons } from './deck';
  import { appearanceOf, tileColors } from '../editor/appearance';
  import ButtonContent from './ButtonContent.svelte';
  import NumberField from '../../components/NumberField.svelte';
  let {
    layout,
    folderId,
    buttons,
    appearance,
    editing,
    running,
    outcomes,
    outcomeMessages,
    reason,
    assetUrls,
    usage,
    now,
    usageStatus,
    canRun,
    invoke,
    remove,
    add,
  }: {
    layout: Layout;
    folderId: string;
    buttons: Button[];
    appearance: Record<string, unknown>;
    editing: boolean;
    running: Record<string, boolean>;
    outcomes: Record<string, string>;
    outcomeMessages: Record<string, string>;
    reason: (button: Button) => string;
    assetUrls: Record<string, string>;
    usage: UsageResponse | null;
    now: number;
    usageStatus: string;
    canRun: (button: Button) => boolean;
    invoke: (button: Button) => void;
    remove: (button: Button) => void;
    add: (cell: number) => void;
  } = $props();
  let selectedRow = $state(0);
  $effect(() => {
    folderId;
    selectedRow = 0;
  });
  const placementState = $derived.by(() => {
    try {
      return { placement: placeButtons(buttons, layout.columns, layout.rows), error: '' };
    } catch (error) {
      return { placement: null, error: error instanceof Error ? error.message : String(error) };
    }
  });
  const count = $derived(placementState.placement?.count ?? 0);
  const paged = $derived(count > 16384);
  const totalRows = $derived(Math.ceil(count / layout.columns));
  const startRow = $derived(paged ? Math.min(selectedRow, totalRows - 1) : 0);
  const window = $derived(
    paged ? gridWindow(buttons, layout.columns, layout.rows, startRow, 128) : null,
  );
  const cells = $derived(
    placementState.error
      ? []
      : window
        ? [
            ...window.crossing.map(([cell, item]) => ({ cell, ...item, covered: false })),
            ...window.cells,
          ]
        : gridCells(buttons, layout.columns, layout.rows),
  );
  function visibleRows(cell: number, rows: number) {
    const relative = Math.floor(cell / layout.columns) - startRow;
    return paged ? Math.min(rows + Math.min(0, relative), 128 - Math.max(0, relative)) : rows;
  }
  function buttonStyle(b: Button) {
    const a = appearanceOf(b);
    return `--button-color:${b.color};--button-foreground:${tileColors(b.color).foreground};--icon-size:${appearanceNumber(a.icon_size, appearanceNumber(appearance.icon_size, 42, 0, 200), 0, 200)}px;`;
  }
</script>

{#if placementState.error}<p role="alert">{t(placementState.error)}</p>{/if}
{#if paged}<div class="row-range" role="group" aria-label={t('ui_grid_row_navigation')}>
    <button
      aria-describedby="grid-row-range"
      disabled={startRow === 0}
      onclick={() => (selectedRow = Math.max(0, startRow - 128))}>{t('ui_previous_rows')}</button
    >
    <NumberField
      label={t('ui_first_visible_row')}
      value={startRow + 1}
      min={1}
      max={totalRows}
      onchange={(value) => (selectedRow = value - 1)}
    />
    <button
      aria-describedby="grid-row-range"
      disabled={startRow + 128 >= totalRows}
      onclick={() => (selectedRow = Math.min(totalRows - 1, startRow + 128))}
      >{t('ui_next_rows')}</button
    >
    <p id="grid-row-range">
      {t('ui_visible_grid_rows', {
        first: startRow + 1,
        last: Math.min(totalRows, startRow + 128),
        total: totalRows,
      })}
    </p>
  </div>{/if}
<!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable fixed-cell decks need a keyboard focus target for arrow-key panning.) -->
<div class="deck-scroll" role="region" aria-label={t('ui_control_deck')} tabindex="0">
  <div
    class="deck-grid all-buttons"
    class:editing
    style:grid-template-columns={`repeat(${layout?.columns ?? 4}, minmax(72px, 1fr))`}
    style:gap={`${appearanceNumber(appearance.gap, 10, 0, 100)}px`}
    style:--button-height={`${appearanceNumber(appearance.button_height, 140, 24, 800)}px`}
    style:--button-radius={`${appearanceNumber(appearance.radius, 8, 0, 100)}px`}
  >
    {#each cells.filter((c) => !c.covered) as slot (slot.cell)}<div
        class="deck-cell"
        data-cell={slot.cell}
        style:grid-column={`${(slot.cell % (layout?.columns ?? 4)) + 1} / span ${slot.columns}`}
        style:grid-row={`${Math.max(1, Math.floor(slot.cell / layout.columns) - startRow + 1)} / span ${visibleRows(slot.cell, slot.rows)}`}
      >
        {#if slot.button}{@const b = slot.button}<button
            class="deck-button button"
            style={buttonStyle(b)}
            class:active={dynamic[b.id]?.active}
            class:blank={b.action.type === 'none'}
            disabled={running[b.id] || !canRun(b)}
            aria-label={dynamic[b.id]?.label ?? b.label}
            aria-describedby={reason(b) || running[b.id] || outcomeMessages[b.id]
              ? `state-${b.id}`
              : undefined}
            title={reason(b) || (outcomeMessages[b.id] ? t(outcomeMessages[b.id]!) : undefined)}
            onclick={() => {
              if (!editing) invoke(b);
            }}
            ><ButtonContent
              button={dynamic[b.id]?.label === undefined
                ? b
                : { ...b, label: dynamic[b.id]!.label! }}
              assetUrl={assetUrls[b.icon.slice(6)]}
              showLabels={appearance.show_labels !== false}
              {usage}
              {now}
            /><small
              class:completed={outcomes[b.id] === 'completed'}
              class:failed={outcomes[b.id] === 'failed'}
              role={outcomes[b.id] === 'completed' ? 'status' : undefined}
              >{running[b.id]
                ? t('ui_running')
                : outcomes[b.id] === 'completed'
                  ? t('ui_completed')
                  : outcomes[b.id] === 'failed'
                    ? t('ui_failed')
                    : (b.action.type === 'usage' ||
                          (b.action.type === 'metric' && b.action.metric !== 'clock')) &&
                        usageStatus === 'stale'
                      ? t('ui_reading_unavailable_stale')
                      : ''}</small
            ></button
          >
          {#if reason(b) || running[b.id] || outcomeMessages[b.id]}<span
              id={`state-${b.id}`}
              class="sr-only"
              >{reason(b) ||
                (running[b.id] ? t('ui_running') : t(outcomeMessages[b.id] ?? ''))}</span
            >{/if}
          {#if editing}<div class="cell-actions">
              <button
                aria-label={t('ui_edit_named_button', { label: b.label })}
                onclick={() => invoke(b)}>{t('ui_edit')}</button
              ><button
                aria-label={t('ui_remove_named_button', { label: b.label })}
                onclick={() => remove(b)}>{t('ui_remove')}</button
              >
            </div>{/if}
        {:else if editing}<button
            class="deck-button add"
            onclick={() => add(slot.cell)}
            aria-label={t('ui_add_button_at_cell', { cell: slot.cell + 1 })}>+</button
          >{:else}<div class="empty-cell" aria-hidden="true"></div>{/if}
      </div>{/each}
  </div>
</div>

<style>
  .row-range {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  small.completed,
  small.failed {
    padding: 2px 6px;
    border-radius: var(--radius-control);
  }
  small.completed {
    color: var(--success-text);
    background: var(--success-surface);
  }
  small.failed {
    color: var(--error-text);
    background: var(--error-surface);
  }
  .deck-scroll {
    width: 100%;
    overflow-x: auto;
  }
  .deck-scroll:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
</style>
