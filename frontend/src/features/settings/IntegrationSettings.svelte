<script lang="ts">
  import type { TranslationKey } from '../../lib/translations';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onMount } from 'svelte';
  import SettingsCard from './ui/SettingsCard.svelte';
  import StatusBadge from './ui/StatusBadge.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import type { Editor } from '../editor/editor.svelte';
  import type { SpotifyConnect, IntegrationStatus, IntegrationState } from '../../lib/contracts';
  import { request } from '../../lib/api/client';
  let {
    editor,
    persist,
    attempt,
  }: {
    editor: Editor;
    persist: () => Promise<void>;
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  let status = $state<IntegrationStatus | null>(null);
  let statusError = $state('');
  let checking = $state(false);
  let changed = $state(false);
  let disposed = false;
  let generation = 0;
  let statusGeneration = 0;
  const stateLabels: Record<IntegrationState, TranslationKey> = {
    not_configured: 'settings.notConfigured',
    not_tested: 'settings.notChecked',
    connected: 'common.connected',
    failed: 'integrations.failed',
    authorization_saved: 'integrations.authorizationStored',
    authorization_required: 'settings.authorizationRequired',
  };
  onMount(() => {
    const started = statusGeneration;
    void request<IntegrationStatus>('integrations/status', 'IntegrationStatus')
      .then((value) => {
        if (!disposed && started === statusGeneration) status = value;
      })
      .catch((error: unknown) => {
        if (!disposed && started === statusGeneration)
          statusError = error instanceof Error ? error.message : String(error);
      });
    return () => {
      disposed = true;
      generation++;
    };
  });
  async function checkObs() {
    if (checking) return;
    checking = true;
    statusGeneration++;
    const started = ++generation;
    statusError = '';
    try {
      await persist();
      if (disposed || started !== generation) return;
      const result = await request<IntegrationStatus>(
        'integrations/obs/check',
        'IntegrationStatus',
        {
          method: 'POST',
        },
      );
      if (disposed || started !== generation) return;
      status = result;
      changed = editor.dirty;
    } catch (error) {
      if (!disposed && started === generation)
        statusError = error instanceof Error ? error.message : String(error);
    } finally {
      checking = false;
    }
  }
  let revealObs = $state(false);
  let revealSpotify = $state(false);
  let spotifyUrl = $state('');
  let connecting = $state(false);
  const change = () => {
    generation++;
    spotifyUrl = '';
    changed = true;
    editor.change();
  };
  async function connectSpotify() {
    if (connecting) return;
    connecting = true;
    const started = ++generation;
    spotifyUrl = '';
    try {
      await persist();
      if (disposed || started !== generation) return;
      const result = await request<SpotifyConnect>('spotify/connect', 'SpotifyConnect', {
        method: 'POST',
      });
      if (!disposed && started === generation) spotifyUrl = result.url;
    } catch (error) {
      if (!disposed && started === generation) throw error;
    } finally {
      connecting = false;
    }
  }
</script>

<div id="settings-integrations" class="settings-page">
  <div class="settings-page-heading"><h2>{t('settings.integrations')}</h2></div>
  {#if checking || connecting}<p id="integration-work-status">
      {checking ? t('integrations.checkingConnection') : t('integrations.preparingAuthorization')}
    </p>{/if}
  {#if statusError}<p role="alert">{t.message(statusError)}</p>{/if}
  {#if changed}<p class="integration-warning">
      {t('integrations.changed')}
    </p>{/if}
  <div class="integration-layout">
    <SettingsCard id="guide-obs" title={t('integrations.obs')} icon="monitor">
      <p aria-label={t('integrations.obsConnectionStatus')} role="status">
        <StatusBadge
          label={status ? t(stateLabels[status.obs]) : t('integrations.loadingConnectionStatus')}
          tone={status?.obs === 'connected'
            ? 'success'
            : status?.obs === 'failed'
              ? 'error'
              : 'neutral'}
        />
      </p>
      <button
        aria-describedby={checking || connecting ? 'integration-work-status' : undefined}
        disabled={checking || connecting}
        data-guide="obs-check"
        onclick={checkObs}
        >{checking
          ? t('integrations.checkingConnection')
          : t('integrations.saveAndCheckObsConnection')}</button
      >
      <label
        >{t('integrations.host')}<input
          bind:value={editor.draft.settings.obs.host}
          oninput={change}
        /></label
      >
      <NumberField
        controls
        label={t('integrations.port')}
        value={editor.draft.settings.obs.port}
        min={1}
        max={65535}
        onchange={(value) => {
          editor.draft.settings.obs.port = value;
          change();
        }}
      /><label
        >{t('integrations.password')}<input
          data-guide="obs-password"
          type={revealObs ? 'text' : 'password'}
          bind:value={editor.draft.settings.obs.password}
          oninput={change}
        /></label
      >
      <button type="button" aria-pressed={revealObs} onclick={() => (revealObs = !revealObs)}
        >{revealObs
          ? t('integrations.hideObsPassword')
          : t('integrations.revealObsPassword')}</button
      >
    </SettingsCard>
    <SettingsCard id="guide-spotify" title={t('integrations.spotify')} icon="spotify">
      <p data-guide="spotify-status" aria-label={t('integrations.spotifyAuthorizationStatus')}>
        <StatusBadge
          label={status
            ? t(stateLabels[status.spotify])
            : t('integrations.loadingAuthorizationStatus')}
        />
      </p>
      <label
        >{t('integrations.clientId')}<input
          bind:value={editor.draft.settings.spotify.client_id}
          oninput={change}
        /></label
      ><label
        >{t('integrations.clientSecret')}<input
          type={revealSpotify ? 'text' : 'password'}
          bind:value={editor.draft.settings.spotify.client_secret}
          oninput={change}
        /></label
      ><label
        >{t('integrations.redirectUri')}<input
          bind:value={editor.draft.settings.spotify.redirect_uri}
          oninput={change}
        /></label
      >
      <button
        type="button"
        aria-pressed={revealSpotify}
        onclick={() => (revealSpotify = !revealSpotify)}
        >{revealSpotify
          ? t('integrations.hideSpotifySecret')
          : t('integrations.revealSpotifySecret')}</button
      >
      <button
        aria-describedby={checking || connecting ? 'integration-work-status' : undefined}
        disabled={connecting || checking}
        data-guide="spotify-connect"
        onclick={() => attempt(connectSpotify)}
        >{connecting
          ? t('integrations.preparingAuthorization')
          : t('integrations.connectSpotify')}</button
      >
      {#if spotifyUrl}<p role="status">
          {t('integrations.settingsSavedContinueToSpotifyToAuthorizeThisHost')}
        </p>
        <a href={spotifyUrl} target="_blank" rel="noopener noreferrer"
          >{t('integrations.continueToSpotify')}</a
        >{/if}
    </SettingsCard>
  </div>
  <p>{t('settings.updatesDisabled')}</p>
</div>

<style>
  .integration-layout {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  @media (max-width: 1100px) {
    .integration-layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .integration-warning {
    color: var(--warning-text);
    background: var(--warning-surface);
    padding: var(--space-3);
    border-radius: var(--radius-control);
  }
</style>
