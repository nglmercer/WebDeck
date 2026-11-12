<script lang="ts">
  import type { Snippet } from 'svelte';
  import { modal } from '../lib/modal';
  let {
    label,
    close,
    children,
    width = '340px',
    nonModal = false,
  }: {
    label: string;
    close: () => void;
    children: Snippet;
    width?: string;
    nonModal?: boolean;
  } = $props();
</script>

<dialog
  use:modal={{ modal: !nonModal }}
  class="deck-menu"
  class:non-modal={nonModal}
  onkeydown={(event) => {
    if (nonModal && event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  }}
  style:--modal-width={width}
  aria-label={label}
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  {@render children()}
</dialog>

<style>
  .non-modal {
    z-index: 24;
  }
  .deck-menu {
    position: fixed;
    inset: 0;
    margin: auto;
    width: min(var(--modal-width), calc(100% - 32px));
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
