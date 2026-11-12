<script lang="ts">
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
  let ttl = $state(900);
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
      }
  }
  function toggle(id: string, capability: Capability, on: boolean) {
    const current = grants[id] ?? [];
    grants[id] = on
      ? [...new Set([...current, capability])]
      : current.filter((value) => value !== capability);
  }
  async function refresh() {
    try {
      const [options, list, permissions] = await Promise.all([
        apiRequest<Profiles>('connection/profiles', 'PermissionProfiles'),
        pairings(),
        apiRequest<PermissionRequests>('connection/permissions', 'PermissionRequests'),
      ]);
      profiles = options;
      upgrades = permissions.requests;
      apply(list);
    } catch (e) {
      if (e instanceof ApiError && (e.status === 401 || e.status === 403)) listening = false;
    }
  }
  async function approve(request: PairingPending) {
    if (tutorial || working) return;
    const selected = (grants[request.id] ?? []).filter((capability) =>
      choices.includes(capability),
    );
    if (!selected.length) return;
    if (profile[request.id] === 'full' && !window.confirm(t('devices.fullControllerHelp'))) return;
    action = 'approve';
    working = request.id;
    try {
      await attempt(async () => {
        apply(
          await approvePairing(request.id, {
            name: request.name,
            capabilities: selected,
            ttl_seconds: ttl,
          }),
        );
      });
    } finally {
      working = '';
    }
  }
  async function reject(request: PairingPending) {
    if (tutorial || working) return;
    action = 'reject';
    working = request.id;
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

{#if shownRequests.length || upgrades.length}
  <aside
    data-guide="pair-approval"
    class="approvals"
    role="region"
    aria-labelledby="pairing-approvals-title"
  >
    <h2 id="pairing-approvals-title">{t('pairing.pendingPairingRequests')}</h2>
    <p class="help">{t(tutorial ? 'guides.library.example' : 'pairing.pairingRequestsHelp')}</p>
    {#each upgrades as upgrade (upgrade.id)}
      <article>
        <h3>{upgrade.device.name}</h3>
        <p>{t('connections.permissionRequestHelp')}</p>
        <p>{upgrade.capabilities.map((c) => t.message(capabilityNames[c] ?? c)).join(', ')}</p>
        <div class="actions">
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
        </div>
      </article>
    {/each}
    {#each shownRequests as request (request.id)}
      <article aria-busy={working === request.id}>
        <div class="who">
          <h3>{request.name}</h3>
          <code>{request.address}</code>
        </div>
        <p class="code"><span class="sr-only">{t('pairing.pairCode')}</span>{request.code}</p>
        <p class="muted">
          {t('devices.deviceExpiry', {
            expiry: new Date(request.expires_at * 1000).toLocaleString(
              document.documentElement.lang,
            ),
          })}
        </p>
        <PermissionProfiles
          bind:selected={grants[request.id]!}
          bind:profile={profile[request.id]!}
          recommended={profiles?.recommended ?? ['read']}
          full={choices}
        />
        <p class="muted" id={`pair-grant-help-${request.id}`}>{t('pairing.pairGrantHelp')}</p>
        <label
          >{t('devices.expiresAfter')}<select data-guide="approval-expiry" bind:value={ttl}
            ><option value={900}>{t('devices.currentSession')}</option><option value={2592000}
              >{t('devices.thirtyDays')}</option
            ><option value={315360000}>{t('devices.untilRevoked')}</option><option value={3600}
              >{t('devices.oneHour')}</option
            ><option value={28800}>{t('devices.eightHours')}</option><option value={86400}
              >{t('devices.oneDay')}</option
            ></select
          ></label
        >
        <div class="actions">
          <button
            class="primary"
            aria-describedby={`pair-grant-help-${request.id}`}
            disabled={working !== '' || !grants[request.id]?.length}
            onclick={() => void approve(request)}
            >{working === request.id && action === 'approve'
              ? t('pairing.approvingDevice')
              : t('pairing.approveDevice')}</button
          >
          <button
            class="danger"
            aria-describedby={`pair-grant-help-${request.id}`}
            disabled={working !== ''}
            onclick={() => void reject(request)}>{t('pairing.reject')}</button
          >
        </div>
      </article>
    {/each}
  </aside>
{/if}

<style>
  .approvals {
    position: fixed;
    top: var(--space-3);
    right: var(--space-3);
    z-index: 25;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: min(360px, calc(100vw - var(--space-3) * 2));
    max-height: calc(100dvh - var(--space-3) * 2);
    overflow: auto;
    padding: var(--space-4);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-dialog);
  }
  h2 {
    margin: 0;
  }
  .help,
  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--font-size-small);
    line-height: 1.5;
  }
  article {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border-subtle);
  }
  .who {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .who h3 {
    margin: 0;
  }
  .who code {
    color: var(--text-muted);
    font-size: var(--font-size-small);
    overflow-wrap: anywhere;
  }
  .code {
    margin: 0;
    font-size: 1.75rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.15em;
  }
  label {
    margin: 0;
  }
  .actions {
    display: flex;
    gap: var(--space-2);
  }
  .actions button {
    flex: 1;
    min-height: 44px;
  }
</style>
