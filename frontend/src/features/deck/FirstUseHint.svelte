<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onMount } from 'svelte';
  let { canEdit }: { canEdit: boolean } = $props();
  let visible = $state(false);
  onMount(() => {
    try {
      visible = localStorage.getItem('webdeck.hint.dismissed') !== '1';
    } catch {
      visible = true;
    }
  });
  function dismiss() {
    visible = false;
    try {
      localStorage.setItem('webdeck.hint.dismissed', '1');
    } catch {
      /* Hint remains dismissible without persistent storage. */
    }
  }
</script>

{#if visible}<div class="hint" role="note" aria-label={t('ui_deck_tips')}>
    <p>{t(canEdit ? 'ui_deck_hint_editor' : 'ui_deck_hint_viewer')}</p>
    <button onclick={dismiss}>{t('ui_got_it')}</button>
  </div>{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: min(460px, calc(100vw - 32px));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: 12px 16px;
    box-shadow: var(--shadow-dialog);
  }
  p {
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--text-muted);
  }
  button {
    flex-shrink: 0;
  }
</style>
