<script lang="ts">
  import { untrack } from 'svelte';
  import TrashIcon from './TrashIcon.svelte';
  import { serializeHeaderRows, type HeaderRow } from './headers';

  /**
   * Reusable name/value row editor (HTTP headers in the fetch form).
   * Row inputs carry `data-nocollect` so form collection skips them;
   * the single hidden carrier submits the joined `Name: value` lines.
   */

  interface Props {
    initial: HeaderRow[];
    dark?: string;
  }

  let { initial, dark = '' }: Props = $props();

  let rows = $state<HeaderRow[]>(untrack(() => initial.map((row) => ({ ...row }))));

  // Trailing-blank invariant: there is always an empty row to type in.
  $effect(() => {
    const last = rows[rows.length - 1];
    if (!last || last.name !== '' || last.value !== '') rows.push({ name: '', value: '' });
  });

  let serialized = $derived(serializeHeaderRows(rows));

  function setRow(index: number, part: 'name' | 'value', text: string): void {
    const row = rows[index];
    if (!row) return;
    rows[index] = { ...row, [part]: text };
  }

  function removeRow(index: number): void {
    rows.splice(index, 1);
  }
</script>

<div class="kv-editor">
  {#each rows as row, i}
    {@const isBlank = i === rows.length - 1}
    <div class="kv-row">
      <input
        class={dark}
        type="text"
        data-nocollect="true"
        placeholder="Name"
        aria-label="Name"
        value={row.name}
        oninput={(e) => setRow(i, 'name', e.currentTarget.value)}
      />
      <input
        class={dark}
        type="text"
        data-nocollect="true"
        placeholder="Value"
        aria-label="Value"
        value={row.value}
        oninput={(e) => setRow(i, 'value', e.currentTarget.value)}
      />
      {#if !isBlank}
        <button
          type="button"
          class="kv-remove"
          aria-label={row.name === '' ? 'Remove row' : `Remove ${row.name}`}
          onclick={() => removeRow(i)}
        >
          <TrashIcon title="Remove" />
        </button>
      {:else}
        <span class="kv-remove kv-spacer" aria-hidden="true"></span>
      {/if}
    </div>
  {/each}
  <input type="hidden" value={serialized} />
</div>

<style>
  .kv-editor {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
  }
  .kv-row {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 4px;
    align-items: center;
  }
  .kv-remove {
    background: transparent;
    border: none;
    padding: 0 2px;
    cursor: pointer;
    color: inherit;
    line-height: 0;
  }
  .kv-spacer {
    display: inline-block;
    width: 20px;
    cursor: default;
  }
</style>
