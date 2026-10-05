<script lang="ts">
  import { capabilityNames } from '../../lib/capability-labels';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onMount, onDestroy } from 'svelte';
  import type { Device, DeviceApproval, Capability } from '../../lib/contracts';
  import { devices as listDevices, approve, revoke } from '../../lib/api/client';
  let { attempt }: { attempt: (work: () => Promise<void>) => Promise<void> } = $props();
  let devices = $state<Device[]>([]),
    grant = $state<DeviceApproval | null>(null),
    deviceName = $state('');
  let grantCapabilities = $state<Capability[]>(['read', 'input']),
    grantTtl = $state(86400);
  let loading = $state(true),
    loadError = $state(''),
    copied = $state(false),
    working = $state(false);
  let destroyed = false;
  let loadGeneration = 0;
  let revoking = $state<Record<string, boolean>>({});
  onDestroy(() => {
    destroyed = true;
    loadGeneration++;
    grant = null;
  });
  async function getDevices() {
    const generation = ++loadGeneration;
    loading = true;
    loadError = '';
    try {
      const result = await listDevices();
      if (!destroyed && generation === loadGeneration) devices = result.devices;
    } catch (e) {
      if (!destroyed && generation === loadGeneration)
        loadError = e instanceof Error ? e.message : String(e);
    } finally {
      if (!destroyed && generation === loadGeneration) loading = false;
    }
  }
  async function addDevice() {
    if (working) return;
    working = true;
    try {
      const approved = await approve({
        name: deviceName,
        capabilities: grantCapabilities,
        ttl_seconds: grantTtl,
      });
      if (destroyed) return;
      grant = approved;
      copied = false;
      await getDevices();
    } finally {
      working = false;
    }
  }
  async function copyToken() {
    const candidate = grant;
    if (!candidate) return;
    await navigator.clipboard.writeText(candidate.token);
    if (!destroyed && grant === candidate) {
      copied = true;
      grant = null;
    }
  }
  async function revokeDevice(id: string) {
    if (revoking[id]) return;
    revoking[id] = true;
    try {
      await revoke(id);
      if (!destroyed) await getDevices();
    } finally {
      if (!destroyed) revoking[id] = false;
    }
  }
  onMount(() => {
    void getDevices();
  });
</script>

<section id="settings-devices">
  <h2>{t('ui_paired_devices')}</h2>
  {#if loading}<p role="status">{t('ui_loading_devices')}</p>{:else if loadError}<p role="alert">
      {t(loadError)}
    </p>
    <button onclick={getDevices}>{t('ui_retry')}</button>{:else if !devices.length}<p>
      {t('ui_no_devices_paired_yet')}
    </p>{/if}
  <label>{t('ui_device_name')}<input bind:value={deviceName} /></label>
  <fieldset>
    <legend>{t('ui_permissions')}</legend
    >{#each ['read', 'input', 'audio', 'window', 'power', 'script', 'network', 'plugin'] as c}<label
        class="check"
        ><input type="checkbox" value={c} bind:group={grantCapabilities} />{t(
          capabilityNames[c] ?? c,
        )}</label
      >{/each}
  </fieldset>
  <label
    >{t('ui_expires_after')}<select bind:value={grantTtl}
      ><option value={900}>{t('ui_15_minutes')}</option><option value={3600}
        >{t('ui_1_hour')}</option
      ><option value={28800}>{t('ui_8_hours')}</option><option value={86400}>{t('ui_1_day')}</option
      ></select
    ></label
  >
  <button
    aria-describedby="device-approval-help"
    disabled={working || !deviceName.trim() || !grantCapabilities.length}
    onclick={() => attempt(addDevice)}>{t('ui_approve_device')}</button
  >
  <p id="device-approval-help">
    {working ? t('ui_approving_device') : t('ui_device_approval_help')}
  </p>
  {#if grant}<p>{t('ui_copy_this_token_now_it_will_not_be_shown_again')}</p>
    <code class="token">{grant.token}</code><button onclick={() => attempt(copyToken)}
      >{copied ? t('ui_copied') : t('ui_copy_token')}</button
    ><button
      onclick={() => {
        grant = null;
      }}>{t('ui_hide_token')}</button
    >{/if}
  {#if copied && !grant}<p role="status">{t('ui_token_copied_keep_it_somewhere_safe')}</p>{/if}
  {#each devices as device}<div class="row">
      <div>
        <strong>{device.name}</strong> ·
        <span id={`device-state-${device.id}`}
          >{device.revoked
            ? t('ui_revoked')
            : revoking[device.id]
              ? t('ui_revoking_device')
              : t('ui_approved')}</span
        >
        <p class="device-details">
          {t('ui_device_expiry', {
            expiry: new Date(device.expires_at * 1000).toLocaleString(
              document.documentElement.lang,
            ),
          })} · {device.capabilities
            .map((capability) => t(capabilityNames[capability] ?? capability))
            .join(', ')}
        </p>
      </div>
      <button
        aria-describedby={`device-state-${device.id}`}
        disabled={device.revoked || revoking[device.id]}
        onclick={() => attempt(() => revokeDevice(device.id))}>{t('ui_revoke')}</button
      >
    </div>{/each}
</section>

<style>
  .token {
    display: block;
    overflow-wrap: anywhere;
    background: var(--canvas);
    padding: 12px;
    font-size: 0.85rem;
  }
</style>
