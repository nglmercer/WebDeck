<script lang="ts">
  import SettingsCard from './ui/SettingsCard.svelte';
  import StatusBadge from './ui/StatusBadge.svelte';
  import { onMount } from 'svelte';
  import { useTranslations } from '../../lib/i18n';
  import { request } from '../../lib/api/http';
  import type { RuntimeSnapshot } from '../../lib/contracts';
  const t = useTranslations();
  let {
    attempt,
    reload,
  }: { attempt: (work: () => Promise<void>) => Promise<void>; reload: () => Promise<void> } =
    $props();
  let status = $state<RuntimeSnapshot | null>(null);
  let failure = $state('');
  let busy = $state(false);
  async function inspect() {
    try {
      status = await request<RuntimeSnapshot>('runtime', 'RuntimeSnapshot');
      failure = '';
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error);
    }
  }
  async function refresh() {
    busy = true;
    try {
      status = await request<RuntimeSnapshot>('runtime/reload', 'RuntimeSnapshot', {
        method: 'POST',
      });
      failure = '';
      await reload();
    } finally {
      busy = false;
    }
  }
  async function toggle(id: string, enabled: boolean) {
    status = await request<RuntimeSnapshot>(
      `runtime/plugins/${encodeURIComponent(id)}`,
      'RuntimeSnapshot',
      { method: 'PUT', body: JSON.stringify({ enabled }) },
    );
    await reload();
  }
  onMount(() => {
    void inspect();
  });
</script>

<div id="settings-runtime" class="settings-page" aria-labelledby="runtime-title">
  <div class="settings-page-heading"><h2 id="runtime-title">{t('runtime.runtime')}</h2></div>
  <SettingsCard id="guide-runtime-status" title={t('runtime.runtimeStatus')} icon="grid">
    {#if status}
      <p>
        <StatusBadge
          label={status.healthy ? t('runtime.runtimeReady') : t('runtime.runtimeFailed')}
          tone={status.healthy ? 'success' : 'error'}
        />
      </p>
      <p>{t('runtime.runtimeCommands', { count: status.commands.length })}</p>
      {#if status.plugins.length}
        <ul class="plugin-list">
          {#each status.plugins as plugin}<li>
              <span
                ><strong>{plugin.id}</strong><small
                  >{plugin.version} · {plugin.backend === 'trusted_process'
                    ? t('runtime.trustedNativePlugin')
                    : t('runtime.sandboxPlugin')}</small
                ></span
              >
              {#if !plugin.id.startsWith('builtin.')}
                <button
                  onclick={() =>
                    attempt(() => toggle(plugin.id, status!.disabled_plugins.includes(plugin.id)))}
                  >{status.disabled_plugins.includes(plugin.id)
                    ? t('runtime.enablePlugin')
                    : t('runtime.disablePlugin')}</button
                >
              {/if}
            </li>{/each}
        </ul>
      {:else}<p>{t('runtime.runtimeNoPlugins')}</p>{/if}
    {/if}
    {#if failure}<p role="alert">{t.message(failure)}</p>{/if}
    <button data-guide="runtime-reload" disabled={busy} onclick={() => attempt(refresh)}
      >{t('runtime.reloadPlugins')}</button
    >
  </SettingsCard>
</div>
