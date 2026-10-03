<script lang="ts">
  import type { Snippet } from 'svelte';
  import { useTranslations } from '../lib/i18n';
  const t = useTranslations();

  let {
    error = $bindable(),
    notice = $bindable(),
    children,
  }: { error: string; notice: string; children?: Snippet } = $props();
</script>

<div class="notifications">
  {@render children?.()}
  {#if error}<div class="banner error" role="alert">
      <span>{t(error)}</span><button aria-label={t('ui_dismiss_error')} onclick={() => (error = '')}
        >×</button
      >
    </div>{/if}
  {#if notice}<div class="banner" role="status">
      <span>{t(notice)}</span><button
        aria-label={t('ui_dismiss_notice')}
        onclick={() => (notice = '')}>×</button
      >
    </div>{/if}
</div>

<style>
  .notifications {
    position: fixed;
    bottom: var(--feedback-bottom, 12px);
    left: 50%;
    transform: translateX(-50%);
    width: max-content;
    max-width: calc(100% - 24px);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 30;
  }
  .banner {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    background: #292929;
    padding: 10px 16px;
    border-radius: var(--radius-control);
    font-size: var(--font-size-small);
  }
  .banner.error {
    background: var(--error-surface);
  }
  .banner button {
    border: 0;
    padding: 0;
    background: transparent;
    min-width: 44px;
    min-height: 44px;
    flex-shrink: 0;
  }
  .banner span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  @media (max-width: 640px), (pointer: coarse) {
    .notifications {
      bottom: var(--feedback-mobile-bottom, 12px);
    }
  }
</style>
