<script lang="ts">
  import type { GuideId } from './guides';
  import { useGuideHelp } from './context';
  import { useTranslations } from '../../lib/i18n';
  import { tutorialActive } from './tour-state';
  const open = useGuideHelp();
  const t = useTranslations();
  let { guide, label }: { guide: GuideId; label: string } = $props();
</script>

{#if open && !$tutorialActive}<button
    type="button"
    class="guide-button"
    aria-label={t('guides.library.contextHelp', { topic: label })}
    title={t('guides.library.contextHelp', { topic: label })}
    onclick={() => open?.(guide)}><span aria-hidden="true">?</span></button
  >{/if}

<style>
  .guide-button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    min-width: 32px;
    min-height: 32px;
    padding: 5px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .guide-button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
