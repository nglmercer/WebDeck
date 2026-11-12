<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Modal from '../../components/Modal.svelte';
  import ChoiceField from './ui/ChoiceField.svelte';
  import StatusBadge from './ui/StatusBadge.svelte';
  import { capabilityNames } from '../../lib/capability-labels';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onMount, onDestroy, untrack } from 'svelte';
  import { tutorialActive } from '../learn/tour-state';
  import PermissionProfiles from '../connections/PermissionProfiles.svelte';
  import { request } from '../../lib/api/client';
  import type {
    DevicePresenceList,
    PermissionProfiles as Profiles,
    DeviceUpdate,
  } from '../../lib/contracts';
  import type { Device, DeviceApproval, Capability } from '../../lib/contracts';
  import { devices as listDevices, approve, revoke } from '../../lib/api/client';
  let {
    attempt,
    tutorial = false,
    guideToken = false,
  }: {
    attempt: (work: () => Promise<void>) => Promise<void>;
    tutorial?: boolean;
    guideToken?: boolean;
  } = $props();
  let presence = $state<DevicePresenceList | null>(null),
    profiles = $state<Profiles | null>(null),
    selected = $state<Device | null>(null);
  let tokenDialog = $state(false);
  $effect(() => {
    if (!guideToken) return;
    const previous = untrack(() => tokenDialog);
    tokenDialog = true;
    return () => {
      tokenDialog = previous;
    };
  });
  let newName = $state(''),
    newCapabilities = $state<Capability[]>([]),
    newTtl = $state(2592000);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let devices = $state<Device[]>([]),
    grant = $state<DeviceApproval | null>(null),
    deviceName = $state('');
  const example: Device = {
    id: 'webdeck-example-phone',
    name: t('ui_guide_local_sample'),
    capabilities: ['read'],
    expires_at: Math.floor(Date.now() / 1000) + 2592000,
    revoked: false,
  };
  const shownDevices = $derived(tutorial ? [example] : devices);
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
    clearTimeout(timer);
    destroyed = true;
    loadGeneration++;
    grant = null;
  });
  async function getDevices() {
    const generation = ++loadGeneration;
    loading = true;
    loadError = '';
    try {
      const [result, state, options] = await Promise.all([
        listDevices(),
        request<DevicePresenceList>('devices/presence', 'DevicePresenceList').catch(() => null),
        request<Profiles>('connection/profiles', 'PermissionProfiles').catch(() => null),
      ]);
      if (!destroyed) {
        presence = state;
        profiles = options;
      }
      if (!destroyed && generation === loadGeneration) devices = result.devices;
    } catch (e) {
      if (!destroyed && generation === loadGeneration)
        loadError = e instanceof Error ? e.message : String(e);
    } finally {
      if (!destroyed && generation === loadGeneration) {
        loading = false;
        clearTimeout(timer);
        timer = setTimeout(() => void getDevices(), 5000);
      }
    }
  }
  function profileName(device: Device) {
    const same = (values: Capability[]) =>
      device.capabilities.length === values.length &&
      values.every((cap) => device.capabilities.includes(cap));
    return same(['read'])
      ? 'ui_profile_view'
      : profiles && same(profiles.full)
        ? 'ui_profile_full'
        : profiles && same(profiles.recommended)
          ? 'ui_profile_deck'
          : 'ui_profile_custom';
  }
  function edit(device: Device) {
    selected = device;
    newName = device.name;
    newCapabilities = [...device.capabilities];
  }
  async function saveDevice() {
    if ($tutorialActive) return;
    if (!selected || working) return;
    if (
      newCapabilities.some(
        (cap) =>
          ['power', 'script', 'network', 'plugin'].includes(cap) &&
          !selected!.capabilities.includes(cap),
      ) &&
      !window.confirm(t('ui_full_controller_help'))
    )
      return;
    working = true;
    try {
      const update: DeviceUpdate = {
        name: newName,
        capabilities: newCapabilities,
        ttl_seconds: newTtl,
      };
      await request(`devices/${encodeURIComponent(selected.id)}`, 'Device', {
        method: 'PATCH',
        body: JSON.stringify(update),
      });
      selected = null;
      await getDevices();
    } finally {
      working = false;
    }
  }
  async function addDevice() {
    if ($tutorialActive) return;
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
    if ($tutorialActive) return;
    const candidate = grant;
    if (!candidate) return;
    await navigator.clipboard.writeText(candidate.token);
    if (!destroyed && grant === candidate) {
      copied = true;
      grant = null;
    }
  }
  async function revokeDevice(id: string) {
    if ($tutorialActive) return;
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
  <div class="devices-heading">
    <h3>{t('ui_paired_devices')}</h3>
    <span>{shownDevices.filter((device) => !device.revoked).length}</span>
  </div>
  {#if !tutorial && loading && !devices.length}<p role="status">
      {t('ui_loading_devices')}
    </p>{:else if !tutorial && loadError}<p role="alert">
      {t(loadError)}
    </p>
    <button onclick={getDevices}>{t('ui_retry')}</button>{:else if !shownDevices.length}<p>
      {t('ui_no_devices_paired_yet')}
    </p>{/if}
  {#if selected}
    <Modal label={t('ui_manage_device')} width="640px" close={() => (selected = null)}>
      <p>
        {selected.capabilities
          .map((capability) => t(capabilityNames[capability] ?? capability))
          .join(', ')}
      </p>
      {#if presence?.devices.find((item) => item.device.id === selected?.id)?.last_seen}<p>
          {t('ui_last_seen')}: {new Date(
            presence.devices.find((item) => item.device.id === selected?.id)!.last_seen * 1000,
          ).toLocaleString()}
        </p>{/if}
      <fieldset disabled={working}>
        <legend>{t('ui_manage_device')}</legend>
        <label>{t('ui_device_name')}<input bind:value={newName} maxlength="128" /></label>
        <PermissionProfiles
          bind:selected={newCapabilities}
          recommended={profiles?.recommended ?? ['read']}
        />
        <ChoiceField
          label={t('ui_trust_duration')}
          value={newTtl}
          options={[
            { value: 900, label: t('ui_current_session') },
            { value: 2592000, label: t('ui_30_days') },
            { value: 315360000, label: t('ui_until_revoked') },
          ]}
          onchange={(value) => (newTtl = value)}
        />
        <button
          disabled={!newName.trim() || !newCapabilities.length}
          onclick={() => attempt(saveDevice)}>{t('ui_save_changes')}</button
        >
        <button onclick={() => (selected = null)}>{t('ui_cancel')}</button>
      </fieldset>
    </Modal>
  {/if}
  {#each shownDevices as device}<article class="device-card">
      <div class="device-identity">
        <div class="device-title">
          <Icon name="devices" size={20} /><strong>{device.name}</strong>
          <span id={`device-state-${device.id}`}
            ><StatusBadge
              label={device.revoked
                ? t('ui_revoked')
                : revoking[device.id]
                  ? t('ui_revoking_device')
                  : presence?.devices.find((p) => p.device.id === device.id)?.online
                    ? t('ui_connected')
                    : t('ui_offline')}
              tone={device.revoked
                ? 'neutral'
                : presence?.devices.find((p) => p.device.id === device.id)?.online
                  ? 'success'
                  : 'neutral'}
            /></span
          >
        </div>
        <p class="device-profile">{t(profileName(device))}</p>
        <p class="device-expiry">
          {t('ui_device_expiry', {
            expiry:
              device.expires_at === 9007199254740991
                ? t('ui_until_revoked')
                : new Date(device.expires_at * 1000).toLocaleDateString(
                    document.documentElement.lang,
                    { dateStyle: 'medium' },
                  ),
          })}
        </p>
      </div>
      <div class="device-actions">
        <button disabled={device.revoked} onclick={() => edit(device)}
          >{t('ui_manage_device')}</button
        >
        <button
          aria-describedby={`device-state-${device.id}`}
          disabled={device.revoked || revoking[device.id]}
          onclick={() => attempt(() => revokeDevice(device.id))}>{t('ui_revoke')}</button
        >
      </div>
    </article>{/each}
  <button class="manual-token" onclick={() => (tokenDialog = true)}
    >{t('ui_use_pairing_token')}</button
  >
  {#if tokenDialog}<Modal
      label={t('ui_use_pairing_token')}
      width="640px"
      close={() => {
        tokenDialog = false;
        grant = null;
      }}
    >
      <div class="heading" data-guide="manual-token">
        <h3>{t('ui_use_pairing_token')}</h3>
        <button
          onclick={() => {
            tokenDialog = false;
            grant = null;
          }}>{t('ui_close')}</button
        >
      </div>
      <label>{t('ui_device_name')}<input bind:value={deviceName} maxlength="128" /></label>
      <PermissionProfiles
        bind:selected={grantCapabilities}
        recommended={profiles?.recommended ?? ['read']}
      />
      <p id="device-approval-help">{t('ui_device_approval_help')}</p>
      <button
        aria-describedby="device-approval-help"
        disabled={working || !deviceName.trim() || !grantCapabilities.length}
        onclick={() => attempt(addDevice)}>{t('ui_approve_device')}</button
      >
      {#if grant}<p>{t('ui_copy_this_token_now_it_will_not_be_shown_again')}</p>
        <code class="token">{grant.token}</code><button onclick={() => attempt(copyToken)}
          >{t('ui_copy_token')}</button
        ><button onclick={() => (grant = null)}>{t('ui_hide_token')}</button>{/if}
      {#if copied}<p role="status">{t('ui_token_copied_keep_it_somewhere_safe')}</p>{/if}
    </Modal>{/if}
</section>

<style>
  #settings-devices {
    min-width: 0;
    margin: 0;
    padding: 20px;
  }
  .devices-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }
  .devices-heading h3 {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0;
  }
  .devices-heading > span {
    color: var(--text-muted);
    font-size: 0.75rem;
  }
  #settings-devices .device-card {
    padding: 14px 0;
    border: 0;
    border-top: 1px solid var(--border-subtle);
    border-radius: 0;
    margin: 0;
  }
  .device-title {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .device-title strong {
    flex: 1;
    font-size: 0.85rem;
    overflow-wrap: anywhere;
  }
  .device-profile {
    margin: 10px 0 4px;
  }
  .device-expiry {
    font-size: 0.75rem;
    margin: 0;
  }
  .manual-token {
    margin-top: 16px;
    background: transparent;
    color: var(--text-muted);
  }
  @media (max-width: 600px) {
    #settings-devices {
      padding: 16px;
    }
  }

  .token {
    display: block;
    overflow-wrap: anywhere;
    background: var(--canvas);
    padding: 12px;
    font-size: 0.85rem;
  }
</style>
