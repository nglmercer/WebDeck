<script lang="ts">
  import type { Snippet } from 'svelte';
  import { modal } from '../lib/modal';
  let { label, close, children }: { label: string; close: () => void; children: Snippet } =
    $props();
</script>

<dialog
  use:modal
  class="deck-menu"
  aria-label={label}
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  {@render children()}
</dialog>

<style>
  .deck-menu {
    position: fixed;
    inset: 0;
    margin: auto;
    width: min(340px, 100%);
    height: max-content;
    max-height: 90dvh;
    overflow: auto;
    background: var(--surface);
    color: inherit;
    border: 1px solid #ffffff25;
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: var(--shadow-dialog);
  }
  .deck-menu :global(p) {
    font-size: 0.8rem;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .deck-menu::backdrop {
    background: #0009;
  }
</style>
