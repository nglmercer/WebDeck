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
    activeTab = $bindable('appearance'),
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
    activeTab?: string;
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
<div class="settings-tabs" role="tablist" aria-label={t('ui_settings_sections')}>
  {#each sections as [id, label]}<button
      role="tab"
      id={`tab-${id}`}
      aria-controls={id === 'connection' ? 'settings-connection' : `panel-${id}`}
      aria-selected={activeTab === id}
      tabindex={activeTab === id ? 0 : -1}
      onclick={() => (activeTab = id)}
      onkeydown={(event) => {
        const index = sections.findIndex(([key]) => key === activeTab);
        const next =
          event.key === 'ArrowRight'
            ? (index + 1) % sections.length
            : event.key === 'ArrowLeft'
              ? (index + sections.length - 1) % sections.length
              : event.key === 'Home'
                ? 0
                : event.key === 'End'
                  ? sections.length - 1
                  : -1;
        if (next >= 0) {
          event.preventDefault();
          activeTab = sections[next]![0];
          document.getElementById(`tab-${activeTab}`)?.focus();
        }
      }}>{t(label)}</button
    >{/each}
</div>
<div class="settings-sections">
  <div
    role="tabpanel"
    id="panel-appearance"
    aria-labelledby="tab-appearance"
    hidden={activeTab !== 'appearance'}
  >
    <AppearanceSettings {editor} {languages} {assets} {attempt} />
  </div>
  <div
    role="tabpanel"
    id="panel-integrations"
    aria-labelledby="tab-integrations"
    hidden={activeTab !== 'integrations'}
  >
    <IntegrationSettings {editor} {persist} {attempt} />
  </div>
  <div
    role="tabpanel"
    id="panel-runtime"
    aria-labelledby="tab-runtime"
    hidden={activeTab !== 'runtime'}
  >
    <RuntimeSettings {attempt} {reload} />
  </div>
  <div
    role="tabpanel"
    id="panel-devices"
    aria-labelledby="tab-devices"
    hidden={activeTab !== 'devices'}
  >
    <DeviceSettings {attempt} />
  </div>
  <div
    role="tabpanel"
    id="panel-backups"
    aria-labelledby="tab-backups"
    hidden={activeTab !== 'backups'}
  >
    <BackupSettings {editor} {reload} {notify} />
  </div>
  <div
    hidden={activeTab !== 'connection'}
    role="tabpanel"
    id="settings-connection"
    aria-labelledby="tab-connection"
  >
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
  </div>
</div>

<style>
  .settings-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: 0 auto var(--space-4);
    max-width: 1000px;
  }
  button[role='tab'] {
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 12px 16px;
    text-decoration: none;
  }
  button[role='tab']:hover {
    background: var(--surface-hover);
  }
  button[role='tab']:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  button[aria-selected='true'] {
    background: var(--accent-surface);
    color: white;
  }
  .settings-sections {
    scroll-margin-top: 16px;
  }
</style>
