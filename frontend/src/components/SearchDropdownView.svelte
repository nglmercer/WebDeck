<script lang="ts">
  import { tick } from 'svelte';

  // Interior of the `<search-dropdown>` custom element. The shell
  // (search-dropdown.ts) owns the element API + event contract and mounts
  // this component into the light DOM, so the global theme CSS
  // (`static/css/components/search-dropdown.css`) cascades in unchanged.
  //
  // No `<style>` block on purpose: all styling hooks (`.sd-search` /
  // `.sd-list` / `.sd-option` + `.active`, `.selected`,
  // `[aria-selected]`) are the pre-existing global classes.

  interface Props {
    /** Host element id; `${hostId || 'sd'}-listbox` ids the listbox. */
    hostId: string;
    /** Initial search-box placeholder (later changes via setAttrs). */
    placeholder: string;
    /** Initial extra class(es) for the search box (later via setAttrs). */
    inputClass: string;
    /** Selection callback; the shell turns it into the change event. */
    onSelect: (value: string) => void;
  }

  let { hostId, placeholder, inputClass, onSelect }: Props = $props();

  let options = $state<string[]>([]);
  let selected = $state<string | null>(null);
  let activeValue = $state<string | null>(null);
  let filter = $state('');
  // Initial-only props by design: `mount` never updates props, so later
  // attribute changes arrive via setAttrs() instead.
  // svelte-ignore state_referenced_locally
  let currentPlaceholder = $state(placeholder);
  // svelte-ignore state_referenced_locally
  let currentInputClass = $state(inputClass);
  let listEl: HTMLDivElement | null = $state(null);

  // svelte-ignore state_referenced_locally
  const listId = `${hostId || 'sd'}-listbox`;
  const visible = $derived(
    options.filter((option) => option.toLowerCase().includes(filter.toLowerCase()))
  );
  const searchClass = $derived(
    currentInputClass === '' ? 'sd-search' : `sd-search ${currentInputClass}`
  );

  function matchesFilter(option: string): boolean {
    return option.toLowerCase().includes(filter.toLowerCase());
  }

  /** Replace the option list (selection kept when still present). */
  export function setOptions(next: string[]): void {
    options = [...next];
    if (selected !== null && !options.includes(selected)) {
      selected = null;
    }
    activeValue = null;
  }

  /** Currently selected value ('' when nothing is selected). */
  export function getValue(): string {
    return selected ?? '';
  }

  /** Select a known option ('' clears); unknown values are ignored. */
  export function setValue(next: string): void {
    if (next !== '' && !options.includes(next)) return;
    selected = next === '' ? null : next;
    activeValue = selected;
  }

  /** Push `placeholder` / `input-class` attribute changes into the view. */
  export function setAttrs(nextPlaceholder: string, nextInputClass: string): void {
    currentPlaceholder = nextPlaceholder;
    currentInputClass = nextInputClass;
  }

  /** Visible option values, in order. */
  export function visibleOptions(): string[] {
    return options.filter(matchesFilter);
  }

  function select(value: string): void {
    selected = value;
    activeValue = value;
    onSelect(value);
  }

  function onSearchKey(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      const first = visible[0];
      if (first !== undefined) select(first);
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (visible.length === 0) return;
      const at = visible.indexOf(activeValue ?? '');
      const next =
        event.key === 'ArrowDown'
          ? visible[(at + 1) % visible.length]
          : visible[(at - 1 + visible.length) % visible.length];
      activeValue = next ?? null;
      void tick().then(() => {
        for (const el of listEl?.querySelectorAll('.sd-option') ?? []) {
          if (el.getAttribute('data-value') === activeValue) {
            el.scrollIntoView?.({ block: 'nearest' });
            break;
          }
        }
      });
    } else if (event.key === 'Escape') {
      filter = '';
      activeValue = null;
    }
  }

  function onListClick(event: MouseEvent): void {
    const option = (event.target as Element).closest?.('.sd-option') ?? null;
    const value = option?.getAttribute('data-value');
    if (value !== null && value !== undefined) select(value);
  }
</script>

<input
  type="text"
  class={searchClass}
  autocomplete="off"
  role="combobox"
  aria-expanded="true"
  aria-autocomplete="list"
  aria-controls={listId}
  placeholder={currentPlaceholder}
  bind:value={filter}
  oninput={() => {
    activeValue = null;
  }}
  onkeydown={onSearchKey}
/>
<!-- svelte-ignore a11y_interactive_supports_focus, a11y_click_events_have_key_events: 1:1 port of the original markup (keyboard handling lives on the search box, as before). -->
<div class="sd-list" role="listbox" id={listId} bind:this={listEl} onclick={onListClick}>
  {#if visible.length === 0}<div class="wd2-sd-empty">No matches — try another search.</div>{/if}
  {#each visible as option}<div
      class="sd-option"
      class:selected={option === selected}
      class:active={option === activeValue && option !== selected}
      role="option"
      data-value={option}
      aria-selected={option === selected ? 'true' : 'false'}>{option}</div>{/each}
</div>
