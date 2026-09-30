<script lang="ts">
  interface Props {
    /** 'alert' renders one acknowledgment button; 'confirm' adds cancel. */
    variant: 'alert' | 'confirm';
    message: string;
    confirmLabel: string;
    cancelLabel: string;
    /** Red primary button for destructive confirms. */
    danger?: boolean;
    onResolve: (confirmed: boolean) => void;
  }

  let {
    variant,
    message,
    confirmLabel,
    cancelLabel,
    danger = false,
    onResolve,
  }: Props = $props();
  let primaryBtn: HTMLButtonElement | null = $state(null);
  let overlayEl: HTMLDivElement | null = $state(null);

  // Dismiss value for non-answers: alerts only acknowledge, confirms
  // default to false (safe side for destructive actions).
  const dismissValue = $derived(variant === 'alert');

  $effect(() => {
    primaryBtn?.focus();
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.stopPropagation();
        onResolve(dismissValue);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  function onOverlayClick(event: MouseEvent): void {
    if (event.target === overlayEl) onResolve(dismissValue);
  }
</script>

<div
  class="wd-dialog-overlay"
  role="presentation"
  bind:this={overlayEl}
  onclick={onOverlayClick}
>
  <div class="wd-dialog" role="alertdialog" aria-modal="true" aria-describedby="wd-dialog-message">
    <p id="wd-dialog-message" class="wd-dialog-message">{message}</p>
    <div class="wd-dialog-actions">
      {#if variant === 'confirm'}
        <button
          type="button"
          class="button wd-dialog-btn"
          data-testid="confirm-cancel"
          onclick={() => onResolve(false)}
        >
          {cancelLabel}
        </button>
      {/if}
      <button
        type="button"
        class="button wd-dialog-btn wd-dialog-primary"
        class:wd-dialog-danger={danger}
        data-testid={variant === 'alert' ? 'alert-ok' : 'confirm-ok'}
        bind:this={primaryBtn}
        onclick={() => onResolve(true)}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
