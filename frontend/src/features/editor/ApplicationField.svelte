<script lang="ts">
  import SelectField from '../../components/SelectField.svelte';
  import { request } from '../../lib/api/client';
  import type { ApplicationList } from '../../lib/contracts';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    label,
    value,
    onchange,
    mode = 'processes',
  }: {
    label: string;
    value: string;
    onchange: (value: string) => void;
    mode?: 'audio' | 'processes' | 'open';
  } = $props();
  let applications = $state<ApplicationList['applications']>([]),
    loading = $state(false),
    unavailable = $state(false),
    generation = 0;
  let controller: AbortController;
  function icon(name: string) {
    return /spotify/i.test(name)
      ? 'spotify'
      : /discord/i.test(name)
        ? 'discord'
        : /obs/i.test(name)
          ? 'broadcast'
          : /firefox/i.test(name)
            ? 'firefox'
            : /chrom|brave|edge/i.test(name)
              ? 'browser'
              : /vlc/i.test(name)
                ? 'vlc'
                : /code|codium/i.test(name)
                  ? 'code'
                  : 'app-window';
  }
  async function load() {
    controller?.abort();
    controller = new AbortController();
    const current = ++generation;
    loading = true;
    unavailable = false;
    try {
      const result = await request<ApplicationList>(
        `applications?kind=${mode === 'audio' ? 'audio' : 'processes'}`,
        'ApplicationList',
        { signal: controller.signal },
      );
      if (current !== generation) return;
      applications = result.applications;
      unavailable = !result.available;
    } catch {
      if (current === generation) unavailable = true;
    } finally {
      if (current === generation) loading = false;
    }
  }
  $effect(() => {
    mode;
    void load();
    return () => {
      generation++;
      controller?.abort();
    };
  });
  const options = $derived(
    Array.from(
      new Map(
        applications
          .map((app) => ({
            value: mode === 'open' ? app.executable || app.value : app.value,
            label: app.name,
            description: app.description,
            icon: icon(app.name + ' ' + app.value),
          }))
          .map((option) => [option.value, option]),
      ).values(),
    ),
  );
</script>

<div class="application-field">
  <SelectField
    {label}
    {value}
    {options}
    {onchange}
    searchable
    allowCustom
    maxLength={mode === 'open' ? 4096 : 256}
    required
    {loading}
    onrefresh={() => void load()}
    placeholder={t('forms.apps.placeholder')}
  />{#if unavailable}<p role="status">{t('forms.apps.unavailable')}</p>{/if}
</div>

<style>
  .application-field {
    min-width: 0;
  }
  .application-field p {
    color: var(--text-muted);
    font-size: 0.75rem;
    line-height: 1.5;
    margin: 6px 0;
  }
</style>
