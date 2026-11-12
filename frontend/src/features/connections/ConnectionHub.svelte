<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { tutorialActive } from '../learn/tour-state';
  import { useTranslations } from '../../lib/i18n';
  import { request } from '../../lib/api/client';
  import { onConnectionChange } from '../../lib/api/realtime';
  import { credentials } from '../../lib/api/http';
  import type {
    ConnectionStatus,
    ConnectionInvitation,
    ConnectionProgress,
  } from '../../lib/contracts';
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import StatusBadge from '../settings/ui/StatusBadge.svelte';
  import SettingsCard from '../settings/ui/SettingsCard.svelte';
  import ChoiceField from '../settings/ui/ChoiceField.svelte';
  import NetworkSettings from '../settings/NetworkSettings.svelte';
  import DeviceSettings from '../settings/DeviceSettings.svelte';
  const t = useTranslations();
  let connected = $state(false);
  let diagnosticsOpen = $state(false);
  async function closeInvitation() {
    if ($tutorialActive) return;
    if (working) return;
    if (invitation)
      await request(
        `connection/invitations/${encodeURIComponent(invitation.id)}`,
        'ConnectionProgress',
        { method: 'DELETE' },
      );
    invitation = null;
    clearTimeout(timer);
    if (qr) URL.revokeObjectURL(qr);
    qr = '';
  }
  onMount(() => onConnectionChange((value) => (connected = value)));
  let {
    attempt,
    guidePart,
    transport = $bindable('http'),
    trigger = 0,
  }: {
    guidePart?: string | undefined;
    trigger?: number;
    transport?: 'http' | 'socket';
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  let status = $state<ConnectionStatus | null>(null),
    invitation = $state<ConnectionInvitation | null>(null);
  let progress = $state('waiting'),
    working = $state(false),
    qr = $state(''),
    expired = $state(false),
    error = $state('');
  let destroyed = false,
    generation = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let refresh: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => {
    destroyed = true;
    generation++;
    clearTimeout(timer);
    clearTimeout(refresh);
    if (qr) URL.revokeObjectURL(qr);
  });
  async function load() {
    try {
      const value = await request<ConnectionStatus>('connection/status', 'ConnectionStatus');
      if (destroyed) return;
      if (!$tutorialActive && status && invitation && status.network.url !== value.network.url) {
        expired = true;
        if (qr) URL.revokeObjectURL(qr);
        qr = '';
        if (progress !== 'approved' && !working && value.listener_started) {
          await request(
            `connection/invitations/${encodeURIComponent(invitation.id)}`,
            'ConnectionProgress',
            { method: 'DELETE' },
          );
          await start();
        }
      }
      status = value;
      if (invitation && !expired) {
        const current = await request<ConnectionProgress>(
          `connection/invitations/${encodeURIComponent(invitation.id)}`,
          'ConnectionProgress',
        );
        if (!destroyed) {
          progress = current.state;
          if (['approved', 'rejected', 'expired'].includes(progress) && qr) {
            URL.revokeObjectURL(qr);
            qr = '';
          }
        }
      }
      error = '';
    } catch (e) {
      if (!destroyed) error = e instanceof Error ? e.message : String(e);
    }
    if (!destroyed) refresh = setTimeout(() => void load(), 5000);
  }
  async function start() {
    if ($tutorialActive) return;
    if (working) return;
    working = true;
    const current = ++generation;
    clearTimeout(timer);
    try {
      const value = await request<ConnectionStatus>('connection/start', 'ConnectionStatus', {
        method: 'POST',
      });
      if (destroyed || current !== generation) return;
      status = value;
      const invite = await request<ConnectionInvitation>(
        'connection/invitations',
        'ConnectionInvitation',
        { method: 'POST' },
      );
      const response = await fetch(
        `/api/v2/connection/invitations/${encodeURIComponent(invite.id)}/qr`,
        { headers: credentials(), cache: 'no-store' },
      );
      if (!response.ok) throw new Error(t('ui_phone_qr_failed'));
      const blob = await response.blob();
      if (destroyed || current !== generation) return;
      if (qr) URL.revokeObjectURL(qr);
      qr = URL.createObjectURL(blob);
      invitation = invite;
      progress = 'waiting';
      expired = false;
      timer = setTimeout(
        () => {
          expired = true;
          if (qr) URL.revokeObjectURL(qr);
          qr = '';
        },
        Math.max(0, invite.expires_at * 1000 - Date.now()),
      );
    } finally {
      if (!destroyed) working = false;
    }
  }
  const guideInvite = $derived(guidePart === 'invite');
  const shownInvitation = $derived(
    guideInvite ? { id: 'example', url: 'https://example.com', expires_at: 0 } : invitation,
  );
  const shownProgress = $derived(guideInvite ? 'waiting' : progress);
  $effect(() => {
    if (guidePart !== 'diagnostics') return;
    const previous = untrack(() => diagnosticsOpen);
    diagnosticsOpen = true;
    return () => {
      diagnosticsOpen = previous;
    };
  });
  let lastTrigger = 0;
  $effect(() => {
    if (trigger > lastTrigger) {
      lastTrigger = trigger;
      void attempt(start);
    }
  });
  onMount(() => void load());
</script>

<div class="settings-page connection-page">
  <div class="connection-heading">
    <div class="settings-page-heading">
      <h2>{t('ui_devices_connections')}</h2>
      <p>{t('ui_connection_intro')}</p>
    </div>
    <StatusBadge
      label={t(status?.listener_started ? 'ui_network_ready' : 'ui_network_off')}
      tone={status?.listener_started ? 'success' : 'neutral'}
    />
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
  <div class="connect-strip">
    <span class="connect-icon"><Icon name="devices" size={25} /></span>
    <div>
      <strong>{t('ui_connect_device')}</strong>
      <p>{t('ui_scan_approve_short')}</p>
    </div>
    <button class="primary" disabled={working} onclick={() => attempt(start)}
      ><Icon name="plus" size={17} />{working ? t('ui_connecting') : t('ui_connect_device')}</button
    >
  </div>
  <div class="connection-columns">
    <DeviceSettings
      {attempt}
      tutorial={guidePart === 'device'}
      guideToken={guidePart === 'token'}
    />
    <div class="settings-stack">
      <NetworkSettings {attempt} />
      <SettingsCard title={t('ui_connection')} icon="layers">
        {#snippet actions()}<span aria-label={t('ui_realtime_connection')}
            ><StatusBadge label={t(connected ? 'ui_connected' : 'ui_offline')} /></span
          >{/snippet}
        <div id="settings-connection">
          <ChoiceField
            hideLabel
            label={t('ui_connection')}
            value={transport}
            options={[
              { value: 'http' as const, label: t('ui_http') },
              { value: 'socket' as const, label: t('ui_realtime') },
            ]}
            onchange={(value) => (transport = value)}
          />
        </div>
      </SettingsCard>
    </div>
  </div>
  <div class="connection-utilities">
    <button onclick={() => (diagnosticsOpen = true)}
      ><Icon name="bug" size={17} />{t('ui_connection_diagnostics')}</button
    >
    {#if typeof Notification !== 'undefined'}<button
        onclick={() =>
          attempt(async () => {
            await Notification.requestPermission();
          })}><Icon name="devices" size={17} />{t('ui_pairing_notifications')}</button
      >{/if}
  </div>
  {#if shownInvitation}<Modal
      nonModal
      label={t('ui_connect_device')}
      width="560px"
      close={() => void attempt(closeInvitation)}
    >
      <div class="heading">
        <h2>{t('ui_connect_device')}</h2>
        <button disabled={working} onclick={() => attempt(closeInvitation)}>{t('ui_close')}</button>
      </div>
      <div class="invitation" aria-live="polite" data-guide="connection-invitation">
        {#if guideInvite}<p class="demo-safe">{t('ui_guide_example')}</p>{/if}
        {#if shownProgress === 'approved'}<p role="status">
            {t('ui_device_connected_success')}
          </p>{:else if shownProgress === 'rejected'}<p>
            {t('ui_pair_rejected')}
          </p>{:else if !guideInvite && (expired || shownProgress === 'expired')}<p>
            {t('ui_pair_expired')}
          </p>{:else}
          <h3>{t('ui_scan_approve')}</h3>
          {#if guideInvite}<div class="invitation-placeholder">
              <Icon name="grid" size={44} />
              <p>{t('ui_guide_invitation_placeholder')}</p>
            </div>{:else if qr}<img
              src={qr}
              alt={t('ui_phone_qr_alt')}
              width="290"
              height="290"
            />{/if}
          <div class="connection-link">
            <p>{t('ui_connection_link')}</p>
            <a href={shownInvitation.url} target="_blank" rel="noreferrer">{shownInvitation.url}</a
            ><button
              onclick={() => attempt(() => navigator.clipboard.writeText(shownInvitation!.url))}
              >{t('ui_copy_link')}</button
            >
          </div>
          <p>{t(shownProgress === 'pending' ? 'ui_waiting_host' : 'ui_pair_compare_code')}</p>
        {/if}
        <button disabled={working || guideInvite} onclick={() => attempt(start)}
          >{t('ui_refresh_connection')}</button
        >
      </div>
    </Modal>{/if}
  {#if diagnosticsOpen}<Modal
      label={t('ui_connection_diagnostics')}
      width="640px"
      close={() => (diagnosticsOpen = false)}
    >
      <div class="heading">
        <h2>{t('ui_connection_diagnostics')}</h2>
        <button onclick={() => (diagnosticsOpen = false)}>{t('ui_close')}</button>
      </div>
      <p data-guide="connection-diagnostics">{t('ui_connect_disclosure')}</p>
      <p>{t('ui_phone_troubleshooting')}</p>
      <p>{t('ui_origin_recovery')}</p>
      {#if status}<dl>
          <dt>{t('ui_computer_address')}</dt>
          <dd>{status.network.settings.address}</dd>
          <dt>{t('ui_phone_port')}</dt>
          <dd>{status.network.settings.port}</dd>
        </dl>
        <ul>
          {#each status.addresses as address}<li>{address.interface}: {address.address}</li>{/each}
        </ul>
      {/if}
    </Modal>{/if}
</div>

<style>
  .invitation-placeholder {
    display: grid;
    place-items: center;
    gap: 12px;
    padding: 24px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-panel);
    color: var(--text-muted);
  }
  .demo-safe {
    color: var(--accent);
  }
  .connection-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
  }
  .connect-strip {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px 20px;
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
  }
  .connect-icon {
    display: flex;
    align-items: center;
    color: var(--text-muted);
  }
  .connect-strip > div {
    flex: 1;
    min-width: 0;
  }
  .connect-strip strong {
    font-size: 0.9rem;
    font-weight: 600;
  }
  .connect-strip p {
    margin: 4px 0 0;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 44px;
  }
  .connection-columns {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
    gap: 20px;
    align-items: start;
  }
  .connection-utilities {
    display: flex;
    gap: 8px 20px;
    flex-wrap: wrap;
    border-top: 1px solid var(--border-subtle);
    padding-top: 12px;
  }
  .connection-utilities button {
    color: var(--text-muted);
    padding: 8px 0;
    background: transparent;
    border: 0;
    text-align: left;
  }
  .invitation {
    text-align: center;
  }
  .invitation img {
    display: block;
    width: 240px;
    max-width: 100%;
    height: auto;
    margin: 16px auto;
  }
  .connection-link {
    text-align: left;
    padding: 12px;
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    margin: 16px 0;
  }
  .connection-link p {
    margin: 0 0 8px;
  }
  .connection-link a {
    display: block;
    font-size: 0.8rem;
    overflow-wrap: anywhere;
    margin-bottom: 8px;
  }
  .heading {
    margin: 0;
  }
  .heading h2 {
    margin: 0;
  }
  dd,
  li {
    overflow-wrap: anywhere;
  }
  @media (max-width: 1100px) {
    .connection-columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (max-width: 600px) {
    .connect-strip {
      flex-wrap: wrap;
      padding: 16px;
      gap: 12px;
    }
    .connect-strip > button {
      width: 100%;
    }
    .connect-icon {
      display: none;
    }
  }
</style>
