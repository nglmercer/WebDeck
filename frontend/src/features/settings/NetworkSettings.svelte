<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { useTranslations } from '../../lib/i18n';
  import { request } from '../../lib/api/client';
  import { credentials } from '../../lib/api/http';
  import type { NetworkSettings, NetworkStatus } from '../../lib/contracts';
  const t = useTranslations();
  let { attempt }: { attempt: (work: () => Promise<void>) => Promise<void> } = $props();
  let settings = $state<NetworkSettings>({ enabled: false, address: '', port: 5000 });
  let status = $state<NetworkStatus | null>(null);
  let loading = $state(true),
    working = $state(false),
    error = $state(''),
    qr = $state('');
  let destroyed = false;
  let generation = 0;
  onDestroy(() => {
    destroyed = true;
    generation++;
    if (qr) URL.revokeObjectURL(qr);
  });
  async function display(value: NetworkStatus) {
    const current = ++generation;
    status = value;
    settings = { ...value.settings };
    if (qr) URL.revokeObjectURL(qr);
    qr = '';
    if (!value.settings.enabled) return;
    const response = await fetch('/api/v2/network/qr', {
      headers: credentials(),
      cache: 'no-store',
    });
    if (!response.ok) throw new Error(t('ui_phone_qr_failed'));
    const blob = await response.blob();
    if (destroyed || current !== generation) return;
    qr = URL.createObjectURL(blob);
  }
  async function load() {
    loading = true;
    error = '';
    try {
      const value = await request<NetworkStatus>('network', 'NetworkStatus');
      if (!destroyed) await display(value);
    } catch (e) {
      if (!destroyed) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (!destroyed) loading = false;
    }
  }
  async function apply() {
    working = true;
    try {
      const value = await request<NetworkStatus>('network', 'NetworkStatus', {
        method: 'PUT',
        body: JSON.stringify(settings),
      });
      if (!destroyed) await display(value);
    } finally {
      if (!destroyed) working = false;
    }
  }
  onMount(() => {
    void load();
  });
</script>

<section>
  <h2>{t('ui_phone_access')}</h2>
  <p>{t('ui_phone_access_help')}</p>
  {#if loading}<p role="status">{t('ui_loading_connection_status')}</p>
  {:else if error}<p role="alert">{error}</p>
    <button onclick={load}>{t('ui_retry')}</button>
  {:else}
    <fieldset disabled={working}>
      <label class="check"
        ><input type="checkbox" bind:checked={settings.enabled} />{t(
          'ui_enable_phone_access',
        )}</label
      >
      <label
        >{t('ui_computer_address')}<input
          bind:value={settings.address}
          placeholder={status?.suggested_address}
        /></label
      >
      <button
        disabled={!status?.suggested_address}
        onclick={() => {
          if (status) settings.address = status.suggested_address;
        }}>{t('ui_use_detected_address')}</button
      >
      <label
        >{t('ui_phone_port')}<input
          type="number"
          min="1"
          max="65535"
          bind:value={settings.port}
        /></label
      >
      <button class="primary" onclick={() => attempt(apply)}
        >{working ? t('ui_applying') : t('ui_apply_phone_access')}</button
      >
      <button onclick={load}>{t('ui_refresh_connection')}</button>
    </fieldset>
    <p>{t('ui_phone_applies_immediately')}</p>
    {#if status?.settings.enabled}
      <p role="status">{t('ui_phone_access_enabled')}</p>
      <a href={status.url} target="_blank" rel="noreferrer">{status.url}</a>
      {#if qr}<img src={qr} alt={t('ui_phone_qr_alt')} width="290" height="290" />{/if}
      <ol>
        <li>{t('ui_phone_pair_step')}</li>
        <li>{t('ui_phone_scan_step')}</li>
        <li>{t('ui_phone_token_step')}</li>
      </ol>
      <p>{t('ui_phone_troubleshooting')}</p>
    {:else}<p role="status">{t('ui_phone_access_disabled')}</p>{/if}
  {/if}
</section>

<style>
  fieldset {
    border: 0;
    padding: 0;
  }
  img {
    display: block;
    max-width: 100%;
    height: auto;
    margin: 16px 0;
  }
  a {
    overflow-wrap: anywhere;
  }
</style>
