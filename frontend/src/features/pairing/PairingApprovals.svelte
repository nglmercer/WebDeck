<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
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
    capabilities = [],
    suggested = [],
    attempt,
  }: {
    capabilities: Capability[];
    suggested: Capability[];
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();

  let requests = $state<PairingPending[]>([]);
  let grants = $state<Record<string, Capability[]>>({});
  let ttl = $state(86400);
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
    return needed.length ? needed : fallback.length ? fallback : choices.slice(0, 1);
  }
  function apply(list: PairingList) {
    requests = list.requests;
    for (const request of requests) if (!grants[request.id]) grants[request.id] = defaults();
  }
  function toggle(id: string, capability: Capability, on: boolean) {
    const current = grants[id] ?? [];
    grants[id] = on
      ? [...new Set([...current, capability])]
      : current.filter((value) => value !== capability);
  }
  async function refresh() {
    try {
      apply(await pairings());
    } catch (e) {
      if (e instanceof ApiError && (e.status === 401 || e.status === 403)) listening = false;
    }
  }
  async function approve(request: PairingPending) {
    if (working) return;
    const selected = (grants[request.id] ?? []).filter((capability) =>
      choices.includes(capability),
    );
    if (!selected.length) return;
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
    if (working) return;
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

{#if requests.length}
  <aside class="approvals" role="region" aria-labelledby="pairing-approvals-title">
    <h2 id="pairing-approvals-title">{t('ui_pending_pairing_requests')}</h2>
    <p class="help">{t('ui_pairing_requests_help')}</p>
    {#each requests as request (request.id)}
      <article aria-busy={working === request.id}>
        <div class="who">
          <h3>{request.name}</h3>
          <code>{request.address}</code>
        </div>
        <p class="code"><span class="sr-only">{t('ui_pair_code')}</span>{request.code}</p>
        <p class="muted">
          {t('ui_device_expiry', {
            expiry: new Date(request.expires_at * 1000).toLocaleString(
              document.documentElement.lang,
            ),
          })}
        </p>
        <fieldset>
          <legend>{t('ui_permissions')}</legend>{#each choices as capability (capability)}<label
              class="check"
            >
              <input
                type="checkbox"
                checked={grants[request.id]?.includes(capability)}
                onchange={(event) => toggle(request.id, capability, event.currentTarget.checked)}
              />{t(capabilityNames[capability] ?? capability)}
            </label>{/each}
        </fieldset>
        <p class="muted" id={`pair-grant-help-${request.id}`}>{t('ui_pair_grant_help')}</p>
        <label
          >{t('ui_expires_after')}<select bind:value={ttl}
            ><option value={900}>{t('ui_15_minutes')}</option><option value={3600}
              >{t('ui_1_hour')}</option
            ><option value={28800}>{t('ui_8_hours')}</option><option value={86400}
              >{t('ui_1_day')}</option
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
              ? t('ui_approving_device')
              : t('ui_approve_device')}</button
          >
          <button
            class="danger"
            aria-describedby={`pair-grant-help-${request.id}`}
            disabled={working !== ''}
            onclick={() => void reject(request)}>{t('ui_reject')}</button
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
  fieldset,
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
