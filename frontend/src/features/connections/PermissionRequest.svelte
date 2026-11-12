<script lang="ts">
  import SwitchField from '../../components/SwitchField.svelte';
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
      message = t('pairing.pairExpired');
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
            ? 'connections.permissionApprovedRetry'
            : value.state === 'rejected'
              ? 'pairing.pairRejected'
              : 'pairing.pairExpired',
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
        message = t('connections.connectionExpired');
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
    <summary>{t('connections.requestPermission')}</summary>
    <p>{t('connections.permissionRequestHelp')}</p>
    {#each available.filter((c) => !capabilities.includes(c)) as capability}<SwitchField
        label={t.message(capabilityNames[capability] ?? capability)}
        checked={selected.includes(capability)}
        onchange={(checked) => {
          selected = checked
            ? [...selected, capability]
            : selected.filter((value) => value !== capability);
        }}
      />{/each}
    <button
      disabled={working || pending || !selected.some((cap) => !capabilities.includes(cap))}
      onclick={() => attempt(ask)}>{t('connections.requestPermission')}</button
    >
    {#if pending}<p role="status">{t('connections.permissionRequested')}</p>{/if}
    {#if message}<p role="status">{t.message(message)}</p>{/if}
  </details>
{/if}
