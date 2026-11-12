<script lang="ts">
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import StatusBadge from './ui/StatusBadge.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import SwitchField from './ui/SwitchField.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { useTranslations, common } from '../../lib/i18n';
  import { request } from '../../lib/api/client';
  import { credentials } from '../../lib/api/http';
  import type { NetworkSettings, NetworkStatus } from '../../lib/contracts';
  const t = useTranslations();
  let { attempt }: { attempt: (work: () => Promise<void>) => Promise<void> } = $props();
  let settings = $state<NetworkSettings>({ enabled: false, address: '', port: 5000 });
  let status = $state<NetworkStatus | null>(null);
  let validPort = $state(true);
  let qrOpen = $state(false);
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
  function closeQr() {
    qrOpen = false;
    generation++;
    if (qr) URL.revokeObjectURL(qr);
    qr = '';
  }
  async function loadQr() {
    const current = ++generation;
    if (qr) URL.revokeObjectURL(qr);
    qr = '';
    if (!status?.settings.enabled) return;
    const response = await fetch('/api/v2/network/qr', {
      headers: credentials(),
      cache: 'no-store',
    });
    if (!response.ok) throw new Error(t('connections.phoneQrFailed'));
    const blob = await response.blob();
    if (destroyed || current !== generation) return;
    qr = URL.createObjectURL(blob);
  }
  async function display(value: NetworkStatus) {
    generation++;
    status = value;
    settings = { ...value.settings };
    if (qr) URL.revokeObjectURL(qr);
    qr = '';
    if (!value.settings.enabled) {
      qrOpen = false;
      return;
    }
    if (qrOpen) await loadQr();
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
    if (!validPort) return;
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

<section class="network-card">
  <header class="network-heading">
    <h3>{t('connections.phoneAccess')}</h3>
    <button
      aria-label={t('connections.refreshConnection')}
      disabled={loading || working}
      onclick={load}><Icon name="refresh" size={18} /></button
    >
  </header>
  {#if loading}<p role="status">{t('integrations.loadingConnectionStatus')}</p>
  {:else if error}<p role="alert">{t.message(error)}</p>
    <button onclick={load}>{t(common.retry)}</button>
  {:else}
    <fieldset disabled={working}>
      <SwitchField
        label={t('connections.enablePhoneAccess')}
        checked={settings.enabled}
        onchange={(value) => (settings.enabled = value)}
      />
      <div class="network-fields">
        <label
          >{t('connections.computerAddress')}<input
            bind:value={settings.address}
            placeholder={status?.suggested_address}
          /></label
        >
        <NumberField
          controls
          onvalidity={(valid) => (validPort = valid)}
          label={t('connections.phonePort')}
          value={settings.port}
          min={1}
          max={65535}
          onchange={(value) => (settings.port = value)}
        />
      </div>
      <div class="network-actions">
        <button class="primary" disabled={!validPort} onclick={() => attempt(apply)}
          >{working ? t(common.applying) : t('connections.applyPhoneAccess')}</button
        >
        <button
          disabled={!status?.suggested_address}
          onclick={() => {
            if (status) settings.address = status.suggested_address;
          }}>{t('connections.useDetectedAddress')}</button
        >
      </div>
    </fieldset>
    <p class="apply-help">{t('connections.networkApplyNote')}</p>
    <div class="network-footer">
      <span role="status"
        ><StatusBadge
          label={t(
            status?.settings.enabled ? 'connections.networkReady' : 'connections.networkOff',
          )}
          tone={status?.settings.enabled ? 'success' : 'neutral'}
        /></span
      >
      {#if status?.settings.enabled}<button
          onclick={() =>
            attempt(async () => {
              qrOpen = true;
              await loadQr();
            })}><Icon name="grid" size={16} />{t('connections.showQr')}</button
        >{/if}
    </div>
  {/if}
  {#if qrOpen && status?.settings.enabled}<Modal
      nonModal
      label={t('connections.showQr')}
      width="440px"
      close={closeQr}
    >
      <div class="heading">
        <h2>{t('connections.showQr')}</h2>
        <button onclick={closeQr}>{t(common.close)}</button>
      </div>
      <div class="network-qr">
        {#if qr}<img src={qr} alt={t('connections.phoneQrAlt')} width="240" height="240" />{:else}<p
            role="status"
          >
            {t('integrations.loadingConnectionStatus')}
          </p>{/if}
        <a href={status.url} target="_blank" rel="noreferrer">{status.url}</a>
      </div>
      <ol>
        <li>{t('connections.phonePairStep')}</li>
        <li>{t('connections.phoneScanStep')}</li>
        <li>{t('connections.phoneTokenStep')}</li>
      </ol>
      <button onclick={load}>{t('connections.refreshConnection')}</button>
    </Modal>{/if}
</section>

<style>
  .network-card {
    margin: 0;
    padding: 20px;
    min-width: 0;
  }
  .network-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .network-heading h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .network-heading > button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 10px;
    background: transparent;
    border: 0;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
  }
  .network-fields {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(136px, 1fr);
    gap: 12px;
    align-items: end;
    margin-bottom: 14px;
  }
  .network-fields label {
    margin-bottom: 0;
  }
  .network-actions,
  .network-footer {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .network-footer {
    justify-content: space-between;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
  }
  .network-footer button {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .apply-help {
    font-size: 0.75rem;
    margin: 12px 0 16px;
  }
  .heading {
    margin: 0;
  }
  .heading h2 {
    margin: 0;
  }
  .network-qr {
    text-align: center;
  }
  .network-qr img {
    display: block;
    max-width: 100%;
    height: auto;
    margin: 16px auto;
  }
  a {
    overflow-wrap: anywhere;
    font-size: 0.8rem;
  }
  @media (max-width: 600px) {
    .network-card {
      padding: 16px;
    }
    .network-fields {
      grid-template-columns: minmax(0, 1fr);
      gap: 0;
    }
    .network-actions {
      margin-top: 16px;
    }
  }
</style>
