<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  import type { InvitationPresentation } from './invitation';
  const t = useTranslations();
  let { invitation }: { invitation: InvitationPresentation } = $props();
  let expanded = $state(false);
  const terminal = $derived(
    ['approved', 'rejected', 'expired'].includes(invitation.progress) || invitation.expired,
  );
</script>

<section
  class="invitation"
  class:detected={invitation.progress === 'pending'}
  data-guide="connection-invitation"
>
  <div class="invitation-heading">
    <h3>{t('pairing.flow.invitation')}</h3>
    <button
      type="button"
      class="icon-button"
      disabled={invitation.working || invitation.tutorial}
      aria-label={t('connections.refreshConnection')}
      onclick={invitation.refresh}><Icon name="refresh" size={20} /></button
    >
  </div>
  {#if invitation.progress === 'pending'}
    <button class="mobile-summary" aria-expanded={expanded} onclick={() => (expanded = !expanded)}
      ><Icon name="grid" size={20} />{t('connections.showQr')}<Icon
        name={expanded ? 'chevron-up' : 'chevron-down'}
        size={18}
      /></button
    >
  {/if}
  <div class="qr-content" class:expanded>
    {#if invitation.tutorial}<div class="placeholder">
        <Icon name="grid" size={64} />
        <p>{t('guides.library.invitationPlaceholder')}</p>
      </div>
    {:else if !terminal && invitation.qr}<img
        src={invitation.qr}
        alt={t('connections.phoneQrAlt')}
        width="240"
        height="240"
      />{/if}
    {#if terminal}
      <div class="result" role="status">
        <Icon name={invitation.progress === 'approved' ? 'check' : 'devices'} size={38} />
        <p>
          {t(
            invitation.progress === 'approved'
              ? 'devices.deviceConnectedSuccess'
              : invitation.progress === 'rejected'
                ? 'pairing.pairRejected'
                : 'pairing.pairExpired',
          )}
        </p>
      </div>
    {:else}
      <a
        class="host"
        aria-label={t('connections.connectionLink')}
        href={invitation.url}
        target="_blank"
        rel="noreferrer">{new URL(invitation.url).host}</a
      >
      <button class="copy" type="button" onclick={invitation.copy} disabled={invitation.tutorial}
        ><Icon name="copy" size={18} />{t('connections.copyLink')}</button
      >
      <p class="hint">
        {t(
          invitation.progress === 'pending'
            ? 'pairing.flow.received'
            : 'connections.scanApproveShort',
        )}
      </p>
    {/if}
  </div>
</section>

<style>
  .invitation {
    min-width: 0;
    max-width: none;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .invitation-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 44px;
  }
  .icon-button {
    width: 44px;
    padding: 8px;
    background: transparent;
  }
  .qr-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
  }
  img {
    display: block;
    width: min(240px, 100%);
    height: auto;
    padding: 12px;
    background: white;
    border-radius: 8px;
  }
  .host {
    color: var(--text-muted);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
  }
  .hint {
    text-align: center;
    max-width: 32ch;
    margin: 0;
  }
  .result,
  .placeholder {
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 16px;
    padding: 32px 12px;
  }
  .mobile-summary {
    display: none;
  }
  @media (max-width: 680px) {
    .detected .invitation-heading {
      display: none;
    }
    .mobile-summary {
      display: flex;
      width: 100%;
      justify-content: space-between;
    }
    .detected .qr-content:not(.expanded) {
      display: none;
    }
    .qr-content {
      padding: 12px 0;
    }
  }
</style>
