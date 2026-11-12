<script lang="ts">
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import SelectField from '../../components/SelectField.svelte';
  import InvitationPanel from '../connections/InvitationPanel.svelte';
  import { connectionInvitation } from '../connections/invitation';
  import { useTranslations } from '../../lib/i18n';
  import { onDestroy } from 'svelte';
  import PermissionProfiles from '../connections/PermissionProfiles.svelte';
  import { request as apiRequest } from '../../lib/api/client';
  import type {
    PermissionProfiles as Profiles,
    PermissionPending,
    PermissionRequests,
  } from '../../lib/contracts';
  import { capabilityNames } from '../../lib/capability-labels';
  import { ApiError, approvePairing, pairings, rejectPairing } from '../../lib/api/client';
  import type { Capability, PairingList, PairingPending } from '../../lib/contracts';
  const t = useTranslations();
  const grantable: Capability[] = [
    'read',
    'input',
    'audio',
    'window',
    'power',
    'script',
    'network',
    'plugin',
  ];
  const pollMs = 5000;
  let {
    tutorial = false,
    capabilities = [],
    suggested = [],
    attempt,
  }: {
    tutorial?: boolean;
    capabilities: Capability[];
    suggested: Capability[];
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();

  let requests = $state<PairingPending[]>([]);
  let grants = $state<Record<string, Capability[]>>({ 'webdeck-example-approval': ['read'] });
  let ttl = $state<Record<string, number>>({});
  let names = $state<Record<string, string>>({});
  let selectedId = $state('');
  let opened = $state(false);
  let now = $state(Date.now());
  let completed = $state(false);
  const visible = $derived(Boolean($connectionInvitation) || tutorial || opened);
  $effect(() => {
    if (!visible) return;
    now = Date.now();
    const clock = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(clock);
  });
  $effect(() => {
    if (!$connectionInvitation) completed = false;
  });
  function remaining(request: PairingPending) {
    const seconds = Math.max(0, Math.ceil(request.expires_at - now / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  }
  function close() {
    if (working) return;
    if ($connectionInvitation) $connectionInvitation.close();
    opened = false;
  }
  let profiles = $state<Profiles | null>(null);
  let upgrades = $state<PermissionPending[]>([]);
  let profile = $state<Record<string, string>>({ 'webdeck-example-approval': 'deck' });
  let working = $state('');
  let action = $state<'approve' | 'reject'>('approve');
  let listening = $state(true);

  const choices = $derived.by(() => {
    const own = new Set<string>(capabilities);
    const available = grantable.filter((capability) => own.has(capability));
    return available.length ? available : grantable;
  });
  function defaults() {
    const needed = [...new Set(suggested)].filter((capability) => choices.includes(capability));
    const fallback = (['read', 'input'] as Capability[]).filter((capability) =>
      choices.includes(capability),
    );
    return (
      profiles?.recommended ??
      (needed.length
        ? [...new Set(['read' as Capability, ...needed])]
        : fallback.length
          ? fallback
          : choices.slice(0, 1))
    );
  }
  const example: PairingPending = {
    id: 'webdeck-example-approval',
    name: t('guides.library.localSample'),
    code: '123456',
    address: '192.0.2.10',
    expires_at: Math.floor(Date.now() / 1000) + 900,
  };
  const shownRequests = $derived(tutorial ? [example] : requests);
  const activeRequest = $derived(
    shownRequests.find((value) => value.id === selectedId) ?? shownRequests[0],
  );
  const notifications = new Map<string, Notification>();
  onDestroy(() => {
    notifications.forEach((notification) => notification.close());
    notifications.clear();
  });
  function apply(list: PairingList) {
    for (const [id, notification] of notifications)
      if (!list.requests.some((request) => request.id === id)) {
        notification.close();
        notifications.delete(id);
      }
    if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {
      for (const request of list.requests)
        if (!grants[request.id] && !notifications.has(request.id)) {
          try {
            const notification = new Notification(t('pairing.pendingPairingRequests'), {
              body: `${request.name} · ${request.code}`,
              tag: `webdeck-pairing-${request.id}`,
            });
            notification.onclick = () => {
              window.focus();
              opened = true;
              notification.close();
            };
            notifications.set(request.id, notification);
          } catch {
            /* Unsupported OS notifications must never block approval. */
          }
        }
    }
    requests = list.requests;
    for (const request of requests)
      if (!grants[request.id]) {
        grants[request.id] = defaults();
        profile[request.id] = 'deck';
        names[request.id] = request.name;
        ttl[request.id] = 900;
      }
  }
  let actionVersion = 0;
  async function refresh() {
    const version = actionVersion;
    try {
      const [options, list, permissions] = await Promise.all([
        apiRequest<Profiles>('connection/profiles', 'PermissionProfiles'),
        pairings(),
        apiRequest<PermissionRequests>('connection/permissions', 'PermissionRequests'),
      ]);
      if (version !== actionVersion || working) return;
      profiles = options;
      upgrades = permissions.requests;
      apply(list);
    } catch (e) {
      if (e instanceof ApiError && (e.status === 401 || e.status === 403)) listening = false;
    }
  }
  async function approve(request: PairingPending) {
    if (tutorial || working || request.expires_at * 1000 <= Date.now()) return;
    const selected = (grants[request.id] ?? []).filter((capability) =>
      choices.includes(capability),
    );
    if (!selected.length || !(names[request.id] ?? request.name).trim()) return;
    if (profile[request.id] === 'full' && !window.confirm(t('devices.fullControllerHelp'))) return;
    action = 'approve';
    working = request.id;
    actionVersion++;
    try {
      await attempt(async () => {
        apply(
          await approvePairing(request.id, {
            name: (names[request.id] ?? request.name).trim(),
            capabilities: selected,
            ttl_seconds: ttl[request.id] ?? 900,
          }),
        );
        completed = true;
      });
    } finally {
      working = '';
    }
  }
  async function reject(request: PairingPending) {
    if (tutorial || working) return;
    action = 'reject';
    working = request.id;
    actionVersion++;
    try {
      await attempt(async () => {
        apply(await rejectPairing(request.id));
      });
    } finally {
      working = '';
    }
  }
  $effect(() => {
    if (!listening) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      await refresh();
      if (!stopped && listening) timer = setTimeout(() => void tick(), pollMs);
    };
    void tick();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  });
</script>

{#if !visible && (shownRequests.length || upgrades.length)}
  <button class="requests-trigger" onclick={() => (opened = true)}
    ><Icon name="devices" size={20} />{t('pairing.pendingPairingRequests')}<span
      >{shownRequests.length + upgrades.length}</span
    ></button
  >
{/if}
{#if visible}
  <Modal
    label={t(
      $connectionInvitation ? 'connections.connectDevice' : 'pairing.pendingPairingRequests',
    )}
    width={$connectionInvitation ? '960px' : '560px'}
    {close}
  >
    <div class="dialog-heading">
      <h2>
        {t($connectionInvitation ? 'connections.connectDevice' : 'pairing.pendingPairingRequests')}
      </h2>
      <button
        class="icon-button"
        aria-label={t('common.close')}
        disabled={Boolean(working) || $connectionInvitation?.working}
        onclick={close}><Icon name="close" size={22} /></button
      >
    </div>
    {#if $connectionInvitation}
      <ol class="steps" aria-label={t('connections.connection')}>
        {#each ['scan', 'confirm', 'connected'] as step, index}
          <li
            class:active={index ===
              ($connectionInvitation.progress === 'approved'
                ? 2
                : activeRequest || $connectionInvitation.progress === 'pending'
                  ? 1
                  : 0)}
          >
            <span>{index + 1}</span>{t.message(`pairing.flow.${step}`)}
          </li>
        {/each}
      </ol>
    {/if}
    <div class="connection-layout" class:with-invitation={Boolean($connectionInvitation)}>
      {#if $connectionInvitation}<InvitationPanel invitation={$connectionInvitation} />{/if}
      <section
        class="approvals"
        data-guide="pair-approval"
        aria-label={t('pairing.pendingPairingRequests')}
      >
        {#each upgrades as upgrade (upgrade.id)}
          <article>
            <h3>{upgrade.device.name}</h3>
            <p>{t('connections.permissionRequestHelp')}</p>
            <p>{upgrade.capabilities.map((c) => t.message(capabilityNames[c] ?? c)).join(', ')}</p>
            <div class="actions">
              <button
                onclick={() =>
                  attempt(async () => {
                    await apiRequest(
                      `connection/permissions/${encodeURIComponent(upgrade.id)}`,
                      'PermissionRequests',
                      { method: 'DELETE' },
                    );
                    await refresh();
                  })}>{t('pairing.reject')}</button
              >
              <button
                class="primary"
                onclick={() =>
                  attempt(async () => {
                    await apiRequest(
                      `connection/permissions/${encodeURIComponent(upgrade.id)}`,
                      'PermissionRequests',
                      { method: 'POST' },
                    );
                    await refresh();
                  })}>{t('pairing.approveDevice')}</button
              >
            </div>
          </article>
        {/each}
        {#if shownRequests.length > 1}<SelectField
            label={t('pairing.pendingPairingRequests')}
            value={activeRequest?.id ?? ''}
            options={shownRequests.map((request) => ({
              value: request.id,
              label: `${request.name} · ${request.code}`,
              icon: 'devices',
            }))}
            onchange={(value) => (selectedId = value)}
          />{/if}
        {#if activeRequest}
          {@const request = activeRequest}
          <article aria-busy={working === request.id}>
            <h3>{t('pairing.flow.received')}</h3>
            <div class="who">
              <Icon name="devices" size={28} />
              <div>
                <label
                  >{t('devices.deviceName')}<input
                    aria-label={t('devices.deviceName')}
                    maxlength="128"
                    value={names[request.id] ?? request.name}
                    oninput={(event) => (names[request.id] = event.currentTarget.value)}
                  /></label
                ><code>{request.address}</code>
              </div>
            </div>
            <p class="compare">{t('pairing.flow.compare')}</p>
            <div class="code">
              <span class="sr-only">{t('pairing.pairCode')}</span>{request.code}
            </div>
            <p class="expiry" role="timer">
              <Icon name="clock" size={16} />{t('pairing.flow.expiresIn', {
                time: remaining(request),
              })}
            </p>
            <PermissionProfiles
              compact
              bind:selected={grants[request.id]!}
              bind:profile={profile[request.id]!}
              recommended={profiles?.recommended ?? ['read']}
              full={choices}
            />
            <div data-guide="approval-expiry">
              <SelectField
                label={t('devices.expiresAfter')}
                value={String(ttl[request.id] ?? 900)}
                options={[
                  { value: '900', label: t('devices.currentSession'), icon: 'clock' },
                  { value: '3600', label: t('devices.oneHour'), icon: 'clock' },
                  { value: '28800', label: t('devices.eightHours'), icon: 'clock' },
                  { value: '86400', label: t('devices.oneDay'), icon: 'clock' },
                  { value: '2592000', label: t('devices.thirtyDays'), icon: 'clock' },
                  { value: '315360000', label: t('devices.untilRevoked'), icon: 'clock' },
                ]}
                onchange={(value) => (ttl[request.id] = Number(value))}
              />
            </div>
            {#if tutorial}<p>{t('guides.library.example')}</p>{/if}
            <div class="actions sticky-actions">
              <button disabled={working !== ''} onclick={() => void reject(request)}
                >{t('pairing.reject')}</button
              >
              <button
                class="primary"
                disabled={working !== '' ||
                  !grants[request.id]?.length ||
                  !(names[request.id] ?? request.name).trim() ||
                  request.expires_at * 1000 <= now}
                onclick={() => void approve(request)}
                >{t(
                  working === request.id && action === 'approve'
                    ? 'pairing.approvingDevice'
                    : 'pairing.approveDevice',
                )}</button
              >
            </div>
          </article>
        {:else if !upgrades.length}
          <div class="empty" role="status">
            <Icon
              name={completed || $connectionInvitation?.progress === 'approved'
                ? 'check'
                : 'devices'}
              size={40}
            />
            <h3>
              {t(
                completed || $connectionInvitation?.progress === 'approved'
                  ? 'devices.deviceConnectedSuccess'
                  : 'pairing.flow.waiting',
              )}
            </h3>
            <p>{t('connections.scanApproveShort')}</p>
          </div>
        {/if}
      </section>
    </div>
  </Modal>
{/if}

<style>
  .dialog-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2,
  h3 {
    margin: 0;
  }
  h2 {
    font-size: 1.2rem;
  }
  h3 {
    font-size: 1rem;
  }
  button {
    min-height: 44px;
  }
  .icon-button {
    width: 44px;
    display: grid;
    place-items: center;
    padding: 8px;
    background: transparent;
  }
  .steps {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    list-style: none;
    padding: 16px 0 24px;
    margin: 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 0.85rem;
  }
  .steps li span {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
  }
  .steps .active {
    color: var(--text);
  }
  .steps .active span {
    background: var(--accent);
    color: var(--accent-text, white);
  }
  .connection-layout {
    display: grid;
    min-width: 0;
  }
  .connection-layout.with-invitation {
    grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr);
    gap: 32px;
    padding-top: 12px;
  }
  .with-invitation .approvals {
    border-left: 1px solid var(--border-subtle);
    padding-left: 32px;
  }
  .approvals {
    max-width: none;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .approvals,
  article {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .who > div {
    flex: 1;
    min-width: 0;
  }
  .who label {
    margin: 0 0 8px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .who input {
    margin-top: 4px;
    width: 100%;
    font-size: 1rem;
    color: var(--text);
  }
  .who code {
    font-size: 0.8rem;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .compare {
    margin: 0;
  }
  .code {
    text-align: center;
    padding: 12px;
    border: 1px solid var(--border);
    background: var(--canvas);
    border-radius: 8px;
    font-size: clamp(1.5rem, 5vw, 2rem);
    font-weight: 700;
    letter-spacing: 0.18em;
    font-variant-numeric: tabular-nums;
  }
  .expiry {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 6px;
    margin: -8px 0 0;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .actions button {
    flex: 1;
  }
  .sticky-actions {
    position: sticky;
    bottom: -20px;
    background: var(--surface);
    border-top: 1px solid var(--border-subtle);
    padding: 16px 0;
    margin-bottom: -20px;
    z-index: 1;
  }
  .empty {
    min-height: 250px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    gap: 12px;
  }
  .empty p {
    margin: 0;
  }
  .requests-trigger {
    position: fixed;
    right: 16px;
    bottom: 88px;
    z-index: 23;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
  }
  .requests-trigger span {
    background: var(--accent);
    border-radius: 20px;
    padding: 2px 8px;
  }
  @media (max-width: 680px) {
    .connection-layout.with-invitation {
      grid-template-columns: minmax(0, 1fr);
      gap: 20px;
    }
    .with-invitation .approvals {
      border-left: 0;
      padding-left: 0;
    }
    .steps {
      gap: 8px;
    }
    .steps li {
      flex-direction: column;
      font-size: 0.75rem;
      gap: 6px;
    }
  }
</style>
