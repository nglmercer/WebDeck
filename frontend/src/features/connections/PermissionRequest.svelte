<script lang="ts">
  import { onDestroy } from 'svelte';
  import { useTranslations } from '../../lib/i18n';
  import { capabilityNames } from '../../lib/capability-labels';
  import { request, ApiError } from '../../lib/api/client';
  import type { Capability, PermissionPending, ConnectionProgress } from '../../lib/contracts';
  const t = useTranslations();
  let {
    capabilities,
    attempt,
    authorizationChanged,
  }: {
    capabilities: Capability[];
    attempt: (work: () => Promise<void>) => Promise<void>;
    authorizationChanged: () => void;
  } = $props();
  const available: Capability[] = [
    'input',
    'audio',
    'window',
    'power',
    'script',
    'network',
    'plugin',
  ];
  let selected = $state<Capability[]>([]),
    pending = $state(false),
    working = $state(false),
    message = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  let destroyed = false;
  onDestroy(() => {
    destroyed = true;
    clearTimeout(timer);
  });
  async function poll(challenge: PermissionPending) {
    if (destroyed || !pending) return;
    if (Date.now() / 1000 >= challenge.expires_at) {
      pending = false;
      message = t('ui_pair_expired');
      return;
    }
    try {
      const value = await request<ConnectionProgress>(
        `device/self/permissions/${encodeURIComponent(challenge.id)}`,
        'ConnectionProgress',
      );
      if (destroyed) return;
      if (value.state !== 'pending') {
        pending = false;
        message = t(
          value.state === 'approved'
            ? 'ui_permission_approved_retry'
            : value.state === 'rejected'
              ? 'ui_pair_rejected'
              : 'ui_pair_expired',
        );
        if (value.state === 'approved') {
          selected = [];
          authorizationChanged();
        }
        return;
      }
    } catch (error) {
      if (destroyed) return;
      if (error instanceof ApiError && error.status === 401) {
        pending = false;
        message = t('ui_connection_expired');
        authorizationChanged();
        return;
      }
    }
    if (!destroyed && pending) timer = setTimeout(() => void poll(challenge), 2000);
  }
  async function ask() {
    if (working || pending || !selected.length) return;
    working = true;
    try {
      const challenge = await request<PermissionPending>(
        'device/self/permissions',
        'PermissionPending',
        {
          method: 'POST',
          body: JSON.stringify({
            capabilities: selected.filter((cap) => !capabilities.includes(cap)),
          }),
        },
      );
      if (destroyed) return;
      pending = true;
      message = '';
      clearTimeout(timer);
      void poll(challenge);
    } finally {
      if (!destroyed) working = false;
    }
  }
</script>

{#if available.some((c) => !capabilities.includes(c))}
  <details data-guide="permission-request">
    <summary>{t('ui_request_permission')}</summary>
    <p>{t('ui_permission_request_help')}</p>
    {#each available.filter((c) => !capabilities.includes(c)) as capability}<label class="check"
        ><input type="checkbox" value={capability} bind:group={selected} />{t(
          capabilityNames[capability] ?? capability,
        )}</label
      >{/each}
    <button
      disabled={working || pending || !selected.some((cap) => !capabilities.includes(cap))}
      onclick={() => attempt(ask)}>{t('ui_request_permission')}</button
    >
    {#if pending}<p role="status">{t('ui_permission_requested')}</p>{/if}
    {#if message}<p role="status">{message}</p>{/if}
  </details>
{/if}
