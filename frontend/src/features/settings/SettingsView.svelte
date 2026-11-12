<script lang="ts">
  import GuideButton from '../learn/GuideButton.svelte';
  import type { GuideId } from '../learn/guides';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { tick } from 'svelte';
  import Icon from '../../components/Icon.svelte';
  import './settings.css';
  import type { Editor } from '../editor/editor.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import IntegrationSettings from './IntegrationSettings.svelte';
  import ConnectionHub from '../connections/ConnectionHub.svelte';
  import RuntimeSettings from './RuntimeSettings.svelte';
  import BackupSettings from './BackupSettings.svelte';
  let {
    guidePart,
    activeTab = $bindable('appearance'),
    connectionRequest = 0,
    editor,
    languages,
    transport = $bindable(),
    assets,
    attempt,
    persist,
    back,
    reload,
    notify,
    learn,
    assetUrls = {},
  }: {
    guidePart?: string | undefined;
    activeTab?: string;
    connectionRequest?: number;
    editor: Editor;
    languages: string[];
    transport: 'http' | 'socket';
    assets: () => Promise<void>;
    attempt: (work: () => Promise<void>) => Promise<void>;
    persist: () => Promise<void>;
    back: () => void;
    reload: () => Promise<void>;
    notify: (message: string) => void;
    learn: () => void;
    assetUrls?: Record<string, string>;
  } = $props();
  let shell: HTMLDivElement;
  let invalid = $state(false);
  async function updateValidity() {
    await tick();
    if (!shell?.isConnected) return;
    invalid = !!shell.querySelector('input:invalid, [aria-invalid="true"]');
  }
  async function validatedPersist() {
    await updateValidity();
    if (invalid) throw new Error(t('ui_correct_invalid_fields'));
    await persist();
  }
  function observeValidity(node: HTMLDivElement) {
    const events = ['input', 'change', 'click'] as const;
    for (const event of events) node.addEventListener(event, updateValidity);
    return {
      destroy() {
        for (const event of events) node.removeEventListener(event, updateValidity);
      },
    };
  }
  const helpTopics: Record<string, GuideId> = {
    appearance: 'appearance',
    integrations: 'integrations',
    devices: 'connections',
    backups: 'backups',
    runtime: 'runtime',
  };
  const sections = [
    ['appearance', 'ui_appearance', 'monitor'],
    ['integrations', 'ui_integrations', 'layers'],
    ['devices', 'ui_devices_connections', 'devices'],
    ['backups', 'ui_backups', 'folder'],
    ['runtime', 'ui_runtime', 'grid'],
  ] as const;
</script>

<div class="settings-shell" bind:this={shell} use:observeValidity>
  <header class="settings-header">
    <div class="settings-heading">
      <button class="settings-back" onclick={back}
        ><Icon name="back" size={18} />{t('ui_back_to_deck')}</button
      >
      <h1>{t('ui_settings')}</h1>
    </div>
    <div class="settings-header-actions">
      <GuideButton guide={helpTopics[activeTab] ?? 'overview'} label={t('ui_settings')} />
      <button
        class="primary"
        data-guide="settings-save"
        aria-describedby="settings-save-status"
        disabled={!editor.dirty || editor.saving || invalid}
        onclick={() => attempt(validatedPersist)}>{t('ui_save_changes')}</button
      >
    </div>
    <p id="settings-save-status" class:unsaved={editor.dirty} aria-live="polite">
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
  </header>
  <div class="settings-layout">
    <nav class="settings-navigation">
      <div class="settings-tabs" role="tablist" aria-label={t('ui_settings_sections')}>
        {#each sections as [id, label, icon]}<button
            role="tab"
            id={`tab-${id}`}
            aria-controls={`panel-${id}`}
            aria-selected={activeTab === id}
            tabindex={activeTab === id ? 0 : -1}
            onclick={() => (activeTab = id)}
            onkeydown={(event) => {
              const index = sections.findIndex(([key]) => key === activeTab);
              const next =
                event.key === 'ArrowRight' || event.key === 'ArrowDown'
                  ? (index + 1) % sections.length
                  : event.key === 'ArrowLeft' || event.key === 'ArrowUp'
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
            }}><Icon name={icon} size={19} /><span>{t(label)}</span></button
          >{/each}
      </div>
      <button class="settings-help" onclick={learn}
        ><Icon name="grid" size={17} />{t('ui_settings_help')}</button
      >
    </nav>
    <div class="settings-sections">
      <div
        role="tabpanel"
        id="panel-appearance"
        aria-labelledby="tab-appearance"
        hidden={activeTab !== 'appearance'}
      >
        <AppearanceSettings {editor} {languages} {assets} {attempt} {assetUrls} />
      </div>
      <div
        role="tabpanel"
        id="panel-integrations"
        aria-labelledby="tab-integrations"
        hidden={activeTab !== 'integrations'}
      >
        <IntegrationSettings {editor} persist={validatedPersist} {attempt} />
      </div>
      <div
        role="tabpanel"
        id="panel-runtime"
        aria-labelledby="tab-runtime"
        hidden={activeTab !== 'runtime'}
      >
        {#if activeTab === 'runtime'}
          <RuntimeSettings {attempt} {reload} />
        {/if}
      </div>
      <div
        role="tabpanel"
        id="panel-devices"
        aria-labelledby="tab-devices"
        hidden={activeTab !== 'devices'}
      >
        <ConnectionHub {guidePart} {attempt} bind:transport trigger={connectionRequest} />
      </div>
      <div
        role="tabpanel"
        id="panel-backups"
        aria-labelledby="tab-backups"
        hidden={activeTab !== 'backups'}
      >
        <BackupSettings tutorialPreview={guidePart === 'preview'} {editor} {reload} {notify} />
      </div>
    </div>
  </div>
</div>
