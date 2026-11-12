<script lang="ts">
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
  const stateLabels: Record<IntegrationState, string> = {
    not_configured: 'ui_not_configured',
    not_tested: 'ui_not_checked',
    connected: 'ui_connected',
    failed: 'ui_connection_failed_check_the_host_port_password_and_obs_websocket_server',
    authorization_saved: 'ui_authorization_stored_live_playback_has_not_been_checked',
    authorization_required: 'ui_authorization_required',
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
  <div class="settings-page-heading"><h2>{t('ui_integrations')}</h2></div>
  {#if checking || connecting}<p id="integration-work-status">
      {checking ? t('ui_checking_connection') : t('ui_preparing_authorization')}
    </p>{/if}
  {#if statusError}<p role="alert">{t(statusError)}</p>{/if}
  {#if changed}<p class="integration-warning">
      {t('ui_integration_settings_changed_save_and_check_again_to_verify_the_host_configurati')}
    </p>{/if}
  <div class="integration-layout">
    <SettingsCard title={t('ui_obs')} icon="monitor">
      <p aria-label={t('ui_obs_connection_status')} role="status">
        <StatusBadge
          label={status ? t(stateLabels[status.obs]) : t('ui_loading_connection_status')}
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
        onclick={checkObs}
        >{checking ? t('ui_checking_connection') : t('ui_save_and_check_obs_connection')}</button
      >
      <label
        >{t('ui_host')}<input bind:value={editor.draft.settings.obs.host} oninput={change} /></label
      >
      <NumberField
        controls
        label={t('ui_port')}
        value={editor.draft.settings.obs.port}
        min={1}
        max={65535}
        onchange={(value) => {
          editor.draft.settings.obs.port = value;
          change();
        }}
      /><label
        >{t('ui_password')}<input
          type={revealObs ? 'text' : 'password'}
          bind:value={editor.draft.settings.obs.password}
          oninput={change}
        /></label
      >
      <button type="button" aria-pressed={revealObs} onclick={() => (revealObs = !revealObs)}
        >{revealObs ? t('ui_hide_obs_password') : t('ui_reveal_obs_password')}</button
      >
    </SettingsCard>
    <SettingsCard title={t('ui_spotify')} icon="spotify">
      <p aria-label={t('ui_spotify_authorization_status')}>
        <StatusBadge
          label={status ? t(stateLabels[status.spotify]) : t('ui_loading_authorization_status')}
        />
      </p>
      <label
        >{t('ui_client_id')}<input
          bind:value={editor.draft.settings.spotify.client_id}
          oninput={change}
        /></label
      ><label
        >{t('ui_client_secret')}<input
          type={revealSpotify ? 'text' : 'password'}
          bind:value={editor.draft.settings.spotify.client_secret}
          oninput={change}
        /></label
      ><label
        >{t('ui_redirect_uri')}<input
          bind:value={editor.draft.settings.spotify.redirect_uri}
          oninput={change}
        /></label
      >
      <button
        type="button"
        aria-pressed={revealSpotify}
        onclick={() => (revealSpotify = !revealSpotify)}
        >{revealSpotify ? t('ui_hide_spotify_secret') : t('ui_reveal_spotify_secret')}</button
      >
      <button
        aria-describedby={checking || connecting ? 'integration-work-status' : undefined}
        disabled={connecting || checking}
        onclick={() => attempt(connectSpotify)}
        >{connecting ? t('ui_preparing_authorization') : t('ui_connect_spotify')}</button
      >
      {#if spotifyUrl}<p role="status">
          {t('ui_settings_saved_continue_to_spotify_to_authorize_this_host')}
        </p>
        <a href={spotifyUrl} target="_blank" rel="noopener noreferrer"
          >{t('ui_continue_to_spotify')}</a
        >{/if}
    </SettingsCard>
  </div>
  <p>{t('ui_automatic_updates_are_disabled_for_this_prerelease')}</p>
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
