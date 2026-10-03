<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  import type { Button, UsageResponse } from '../../lib/contracts';
  import { appearanceOf } from '../editor/appearance';
  import { metricReading } from './deck';
  let {
    button,
    assetUrl,
    showLabels = true,
    usage = null,
    now = Date.now(),
  }: {
    button: Button;
    assetUrl?: string | undefined;
    showLabels?: boolean;
    usage?: UsageResponse | null;
    now?: number;
  } = $props();
</script>

{#if button.icon.startsWith('asset:')}
  {#if assetUrl}<img src={assetUrl} alt="" />{:else}<span class="button-icon" aria-hidden="true"
      >▧</span
    >{/if}
{:else}<span class="button-icon" aria-hidden="true">{button.icon}</span>{/if}
<span class:hidden-label={!showLabels || appearanceOf(button).show_label === false}
  >{button.label}</span
>
{#if button.action.type === 'metric' || button.action.type === 'usage'}{@const reading =
    metricReading(button, usage?.usage, now)}
  {#if !showLabels}<span class="metric-title"
      >{typeof appearanceOf(button).metric_label === 'string'
        ? appearanceOf(button).metric_label
        : button.label}</span
    >{/if}
  <strong class="metric-value"
    >{reading.text === 'Unavailable' ? t('ui_metric_no_data') : reading.text}</strong
  >
  {#if reading.percent !== undefined}<progress
      max="100"
      value={reading.percent}
      aria-label={t('ui_reading_usage', { label: button.label })}
    ></progress>{/if}
{/if}
