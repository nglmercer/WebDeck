<script lang="ts">
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

<section id="settings-runtime" aria-labelledby="runtime-title">
  <h2 id="runtime-title">{t('ui_runtime')}</h2>
  {#if status}
    <p>
      {t('ui_runtime_status')}: {status.healthy ? t('ui_runtime_ready') : t('ui_runtime_failed')}
    </p>
    <p>{t('ui_runtime_commands', { count: status.commands.length })}</p>
    {#if status.plugins.length}
      <ul>
        {#each status.plugins as plugin}<li>
            {plugin.id}
            {plugin.version} — {plugin.backend === 'trusted_process'
              ? t('ui_trusted_native_plugin')
              : t('ui_sandbox_plugin')}
            {#if !plugin.id.startsWith('builtin.')}
              <button
                onclick={() =>
                  attempt(() => toggle(plugin.id, status!.disabled_plugins.includes(plugin.id)))}
                >{status.disabled_plugins.includes(plugin.id)
                  ? t('ui_enable_plugin')
                  : t('ui_disable_plugin')}</button
              >
            {/if}
          </li>{/each}
      </ul>
    {:else}<p>{t('ui_runtime_no_plugins')}</p>{/if}
  {/if}
  {#if failure}<p role="alert">{failure}</p>{/if}
  <button disabled={busy} onclick={() => attempt(refresh)}>{t('ui_reload_plugins')}</button>
</section>
