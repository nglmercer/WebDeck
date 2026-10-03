<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { onMount } from 'svelte';
  import { onConnectionChange } from '../../lib/api/realtime';
  import type { Editor } from '../editor/editor.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import IntegrationSettings from './IntegrationSettings.svelte';
  import DeviceSettings from './DeviceSettings.svelte';
  import RuntimeSettings from './RuntimeSettings.svelte';
  import BackupSettings from './BackupSettings.svelte';
  let {
    editor,
    languages,
    transport = $bindable(),
    assets,
    attempt,
    persist,
    back,
    reload,
    notify,
  }: {
    editor: Editor;
    languages: string[];
    transport: 'http' | 'socket';
    assets: () => Promise<void>;
    attempt: (work: () => Promise<void>) => Promise<void>;
    persist: () => Promise<void>;
    back: () => void;
    reload: () => Promise<void>;
    notify: (message: string) => void;
  } = $props();
  let connected = $state(false);
  onMount(() => onConnectionChange((value) => (connected = value)));
  const sections = [
    ['appearance', 'ui_appearance'],
    ['integrations', 'ui_integrations'],
    ['devices', 'ui_devices'],
    ['backups', 'ui_backups'],
    ['runtime', 'ui_runtime'],
    ['connection', 'ui_connection'],
  ] as const;
</script>

<div class="heading">
  <h1>{t('ui_settings')}</h1>
  <div class="row">
    <button onclick={back}>{t('ui_back_to_deck')}</button><button
      class="primary"
      aria-describedby="settings-save-status"
      disabled={!editor.dirty || editor.saving}
      onclick={() => attempt(persist)}>{t('ui_save_changes')}</button
    >
  </div>
</div>
<p id="settings-save-status">
  {editor.saving
    ? t('ui_saving')
    : editor.conflict
      ? t('ui_conflict_draft_preserved')
      : editor.failure
        ? t('ui_save_failed_draft_preserved')
        : editor.dirty
          ? t('ui_unsaved_changes')
          : t('ui_all_changes_saved')}
</p>
<nav aria-label={t('ui_settings_sections')}>
  {#each sections as [id, label]}<a
      href={`#settings-${id}`}
      onclick={(event) => {
        event.preventDefault();
        document.getElementById(`settings-${id}`)?.scrollIntoView({ block: 'start' });
      }}>{t(label)}</a
    >{/each}
</nav>
<div class="settings-sections">
  <AppearanceSettings {editor} {languages} {assets} {attempt} />
  <IntegrationSettings {editor} {persist} {attempt} />
  <RuntimeSettings {attempt} {reload} />
  <DeviceSettings {attempt} />
  <BackupSettings {editor} {reload} {notify} />
  <section id="settings-connection" aria-labelledby="connection-title">
    <h2 id="connection-title">{t('ui_advanced_connection')}</h2>
    <p aria-label={t('ui_realtime_connection')}>
      {connected ? t('ui_connected') : t('ui_offline')}
    </p>
    <label
      >{t('ui_connection')}<select bind:value={transport}
        ><option value="http">{t('ui_http')}</option><option value="socket"
          >{t('ui_realtime')}</option
        ></select
      ></label
    >
  </section>
</div>

<style>
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: 0 auto var(--space-4);
    max-width: 1000px;
  }
  a {
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 12px 16px;
    text-decoration: none;
  }
  a:hover {
    background: var(--surface-hover);
  }
  a:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  .settings-sections {
    scroll-margin-top: 16px;
  }
</style>
