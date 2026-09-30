<script lang="ts">
  import DevicesPanel from '../features/security/DevicesPanel.svelte';
  import Collapse from '../components/Collapse.svelte';
  import SectionIcon from '../components/SectionIcon.svelte';
  import ModalShell from '../components/studio/ModalShell.svelte';
  import ColorField from '../components/ColorField.svelte';
  import NumberField from '../components/NumberField.svelte';
  import SelectField from '../components/SelectField.svelte';
  import SwitchField from '../components/SwitchField.svelte';
  import TextField from '../components/TextField.svelte';
  import StudioSteps from '../components/studio/StudioSteps.svelte';
  import StudioTabs from '../components/studio/StudioTabs.svelte';
  import { tx } from '../components/studio/labels';
  import { submitSettings } from '../features/settings/submit';
  import { send_data } from '../app/send';
  import { hide_modal } from '../app/modals';
  import { text } from '../framework/i18n';
  import { asString, get, type BootContext } from '../framework/types';
  import BackgroundsPanel from './BackgroundsPanel.svelte';
  import ThemesPanel from './ThemesPanel.svelte';
  import { configData } from './config';

  interface Props {
    ctx: BootContext;
  }

  let { ctx }: Props = $props();

  // Render-once by design: renderApp mounts a single boot context, so this
  // intentionally captures the initial prop values. No <style> block on
  // purpose — the global theme stylesheets must keep cascading into this
  // light DOM, exactly as with the previous innerHTML render.
  // svelte-ignore state_referenced_locally
  const config = configData(ctx);

  // Library state (was legacy DOM scraping): the ordered theme list and
  // background list. The hidden handler inputs below serialize them for
  // the config-form save; the panels mutate through bindings.
  // svelte-ignore state_referenced_locally
  let themes = $state(config.themes);
  // svelte-ignore state_referenced_locally
  let backgrounds = $state(config.backgrounds);

  // svelte-ignore state_referenced_locally
  const steps = [
    { id: 'settings', label: text('settings') },
    { id: 'visuals', label: text('visuals') },
    { id: 'experimental', label: text('experimental') },
  ];
  let step = $state('settings');
  let libTab = $state('themes');

  // svelte-ignore state_referenced_locally
  const libTabs = [
    { id: 'themes', label: text('themes_menu_title') },
    { id: 'backgrounds', label: text('random_bg_menu_title') },
  ];

  let saving = $state(false);
  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving) return;
    saving = true;
    try { await submitSettings(event.currentTarget as HTMLFormElement); }
    finally { saving = false; }
  }
  function rotatePortrait(value: string): void {
    const input = document.getElementById('portrait-rotate');
    if (input instanceof HTMLInputElement) input.value = value;
  }

  function bypassFirewall(): void {
    send_data('/bypass-windows-firewall');
  }

  /** Library panels live in the main column — no back-button navigation. */
  function openLibrary(tab: string): void {
    step = 'library';
    libTab = tab;
  }
</script>

<ModalShell
  containerClass="modal-container {config.dark}"
  containerId="modal-container"
  contentClass="modal-content {config.dark}"
  headerClass="modal-header bold"
  titleClass="config-modal"
  title={text('configuration')}
  badge="v{config.version}"
  closeClass="modal-close"
  closeIconClass="config-modal"
  dark={config.dark}
  labelledBy="config-title"
>
    <div class="wd2-version-row">
      <a class={config.dark} id="version" href="https://github.com/Lenochxd/WebDeck/releases/tag/v{config.version}" target="_blank" title={text('see_patchnotes')} aria-label={text('see_patchnotes')}>
        v{config.version}
      </a>
      <a class={config.dark} id="version" href="https://github.com/Lenochxd/WebDeck/issues/new/choose" target="_blank" aria-label={text('github_issue')}>
        {text('github_issue')}
      </a>
      <span style="flex: 1;"></span>
      <button class="button modal-button" id="editorButton"> [Q] {text('enter_editor_mode')} </button>
    </div>
    <div class="modal-main">
      <div class="config-container {config.dark}" id="config-container" style="display: block;">
        <form id="config-form" class="config-form" onsubmit={save} aria-busy={saving}>
          <div class="wd2-config-body">
            <div class="wd2-side">
              <StudioSteps {steps} selected={step} onSelect={(id) => (step = id)} />
              <div class="wd2-lib">
                <h2 class="wd2-lib-title">{tx('studio_library', 'Themes & backgrounds')}</h2>
                <button
                  type="button"
                  class="wd2-lib-btn"
                  data-testid="lib-tab-themes"
                  aria-current={step === 'library' && libTab === 'themes' ? 'true' : 'false'}
                  onclick={() => openLibrary('themes')}
                >
                  <SectionIcon name="swatch" />
                  <span>{text('themes_menu_title')}</span>
                </button>
                <button
                  type="button"
                  class="wd2-lib-btn"
                  data-testid="lib-tab-backgrounds"
                  aria-current={step === 'library' && libTab === 'backgrounds' ? 'true' : 'false'}
                  onclick={() => openLibrary('backgrounds')}
                >
                  <SectionIcon name="image" />
                  <span>{text('random_bg_menu_title')}</span>
                </button>
              </div>
            </div>
            <div class="wd2-config-main">
              <div class="setting-category settings {config.dark}" hidden={step !== 'settings'}>
                <h1 class="config-title"> {text('settings')} </h1>
                <Collapse id="settings-general" title={text('settings_group_general')} icon="sliders" open={true}>
                  <SelectField containerClass="language" id="language" name="settings.language" label={text('language')} options={config.langOptions} />
                  <SwitchField dark={config.dark} containerClass="windows-startup" label={text('config-windows_startup')} id="windows-startup" name="settings.windows_startup" checked={config.windowsStartup} extraClass={config.exeExtraClass} />
                  <SwitchField dark={config.dark} containerClass="auto-updates" label={text('auto_updates')} id="auto-updates" name="settings.auto_updates" checked={config.autoUpdates} extraClass={config.exeExtraClass} />
                </Collapse>
                <Collapse
                  id="settings-soundboard"
                  title={text('soundboard')}
                  icon="speaker"
                  info={{ href: text('link_soundboard'), title: text('soundboard_tutorial'), iconSlot: config.infoSlotId }}
                >
                  <SwitchField dark={config.dark} containerClass="toggle-soundboard" label={text('toggle_soundboard')} id="toggle_soundboard" name="settings.soundboard.enabled" checked={config.soundboardEnabled} />
                  <SelectField containerClass={config.audioInput.containerClass} id={config.audioInput.id} name={config.audioInput.name} label={config.audioInput.label} options={config.audioInput.options} />
                  <SelectField containerClass={config.audioOutput.containerClass} id={config.audioOutput.id} name={config.audioOutput.name} label={config.audioOutput.label} options={config.audioOutput.options} />
                  <SwitchField dark={config.dark} containerClass="ear-soundboard" label={text('ear_soundboard')} id="ear_soundboard" name="settings.ear_soundboard" checked={config.earSoundboard} />
                </Collapse>
                <Collapse
                  id="settings-spotify"
                  title={text('spotify_api')}
                  icon="music"
                  info={{ href: text('link_spotify'), title: text('spotify_tutorial'), iconSlot: config.infoSlotId }}
                >
                  <TextField dark={config.dark} cls="spotify-setting" label={text('username')} labelFor="spotify-username" id="spotify-username" name="settings.spotify_api.username" value={config.spotifyUsername} />
                  <TextField dark={config.dark} cls="spotify-setting" label="Client ID" id="spotify-client_id" name="settings.spotify_api.client_id" value={config.spotifyClientId} />
                  <TextField dark={config.dark} cls="spotify-setting" label="Client Secret" id="spotify-client_secret" name="settings.spotify_api.client_secret" value={config.spotifyClientSecret} password={true} toggleId="show-password-spotify" />
                </Collapse>
                <Collapse
                  id="settings-obs"
                  title={text('obs_studio')}
                  icon="video"
                  info={{ href: text('link_obs'), title: text('obs_tutorial'), iconSlot: config.infoSlotId }}
                >
                  <TextField dark={config.dark} cls="obs-setting" label={text('host')} id="obs-HOST" name="settings.obs.host" value={config.obsHost} />
                  <TextField dark={config.dark} cls="obs-setting" label={text('port')} id="obs-PORT" name="settings.obs.port" value={config.obsPort} />
                  <TextField dark={config.dark} cls="obs-setting" label={text('password')} id="obs-PASSWORD" name="settings.obs.password" value={config.obsPassword} password={true} toggleId="show-password-obs" />
                </Collapse>
              </div>
              <div class="setting-category visuals {config.dark}" hidden={step !== 'visuals'}>
                <h1 class="config-title"> {text('visuals')} </h1>
                <Collapse id="visuals-grid" title={text('settings_group_grid_layout')} icon="grid" open={true}>
                  <div class="setting gridsize">
                    <p> {text('gridsize')} </p>
                    <div class="gridsize-container">
                      <div class="gridsize-height wd2-field">
                        <label class="wd2-label" for="gridsize-height"> {text('height')} </label>
                        <NumberField dark={config.dark} id="gridsize-height" name="front.height" value={config.height} min="1" />
                      </div>
                      <div class="gridsize-width wd2-field">
                        <label class="wd2-label" for="gridsize-width"> {text('width')} </label>
                        <NumberField dark={config.dark} id="gridsize-width" name="front.width" value={config.width} min="1" />
                      </div>
                    </div>
                  </div>
                  <div class="setting portrait-rotate">
                    <label for="portrait-rotate"> {text('portrait_rotate')} </label>
                    <div id="portrait-rotate-setting-container">
                      <NumberField dark={config.dark} id="portrait-rotate" name="front.portrait_rotate" value={config.portraitRotate} defaultValue="90" min="0" required={true} style="width: 75px;" />
                      <div class="deg">&deg;</div>
                      <button type="button" onclick={() => rotatePortrait('270')} class="button">
                        270&deg;
                      </button>
                      <button type="button" onclick={() => rotatePortrait('90')} class="button">
                        90&deg;
                      </button>
                      <button type="button" onclick={() => rotatePortrait('0')} class="button">
                        0&deg;
                      </button>
                    </div>
                  </div>
                </Collapse>
                <Collapse id="visuals-theme" title={text('settings_group_theme_background')} icon="image">
                  <div class="setting themes">
                    <label for="themes"> {text('themes')} </label>
                    <button type="button" id="setting-themes" class={config.dark} onclick={() => openLibrary('themes')}>
                      {text('open_theme_menu')}
                    </button>
                    <input type="text" name="front.themes" id="choose-themes-handler" class="invisible" value={JSON.stringify(themes)} />
                  </div>
                  <div class="setting background">
                    <label for="background"> {text('backgrounds')} </label>
                    <button type="button" id="setting-background" class={config.dark} onclick={() => openLibrary('backgrounds')}>
                      {text('open_background_image_menu')}
                    </button>
                    <input type="text" name="front.background" id="choose-background-handler" class="invisible" value={JSON.stringify(backgrounds)} />
                  </div>
                </Collapse>
                <Collapse id="visuals-appearance" title={text('settings_group_appearance')} icon="eye">
                  <SwitchField dark={config.dark} containerClass={config.dark} label={text('dark_theme')} id="dark-theme" name="front.dark_theme" checked={config.darkTheme} />
                  <SwitchField dark={config.dark} containerClass="show-names" label={text('show_btn_names')} id="show-names" name="front.show_names" checked={config.showNames} />
                  <ColorField dark={config.dark} containerClass="names-color-input-container" colorClass="names-color-input" colorId="names-color-input" hexClass="names-color-setting" hexId="names-color-hex" hexName="front.names_color" placeholder="Button names color (HEX)" value={config.namesColor} />
                  <SwitchField dark={config.dark} containerClass="edit-buttons-color" label={text('edit_btn_color')} id="edit_buttons_color" name="front.edit_buttons_color" checked={config.editButtonsColor} />
                  <ColorField dark={config.dark} containerClass="buttons-color-input-container" colorClass="buttons-color-input" colorId="buttons-color-input" hexClass="buttons-color-setting" hexId="buttons-color-hex" hexName="front.buttons_color" placeholder="Button default color (HEX)" value={config.buttonsColor} />
                </Collapse>
              </div>
              <div class="setting-category experimental {config.dark}" hidden={step !== 'experimental'}>
                <h1 class="config-title"> {text('experimental')} </h1>
                <DevicesPanel />
                <Collapse id="experimental-usage" title={text('settings_group_usage')} icon="chart" open={true}>
                  <div class="setting usage-reload-time">
                    <NumberField dark={config.dark} id="usage-reload-time" name="front.computer_usage_reload_time" label={text('usage_btn_reload_time')} value={config.reloadTime} defaultValue="3000" min="0" required={true} />
                  </div>
                  <SelectField containerClass="gpu_method" id="gpu_method" name="settings.gpu_method" label={text('gpu_usage_method')} options={config.gpuOptions} />
                  <SwitchField dark={config.dark} containerClass="optimized-usage-display" label={text('optimized_usage_display')} id="optimized_usage_display" name="settings.optimized_usage_display" checked={config.optimizedUsage} />
                </Collapse>
                <Collapse id="experimental-advanced" title={text('settings_group_advanced')} icon="flask">
                  <SwitchField dark={config.dark} containerClass="open-settings-in-integrated-browser" label={text('open_settings_in_integrated_browser')} id="open_settings_in_integrated_browser" name="settings.open_settings_in_integrated_browser" checked={config.openInBrowser} />
                  <SwitchField dark={config.dark} containerClass="show-console" label={text('show_console')} id="show_console" name="settings.show_console" checked={config.showConsole} />
                  <div class="setting automatic-firewall-bypass">
                    <button type="button" onclick={bypassFirewall} class="button" id="authorize_windows_firewall">
                      {text('authorize_windows_firewall')}
                    </button>
                  </div>
                  <SwitchField dark={config.dark} containerClass="automatic-firewall-bypass-toggle" label={text('automatic_firewall_bypass')} id="automatic_firewall_bypass" name="settings.automatic_firewall_bypass" checked={config.firewallBypass} />
                  <SwitchField dark={config.dark} containerClass="fix-stop-soundboard" label={text('fix_stop_soundboard')} id="fix_stop_soundboard" name="settings.fix_stop_soundboard" checked={config.fixSoundboard} />
                  <SwitchField dark={config.dark} containerClass="dev-mode" label={text('dev_mode')} id="dev_mode" name="settings.dev_mode" checked={config.devMode} />
                </Collapse>
              </div>
              <div class="setting-category library {config.dark}" hidden={step !== 'library'}>
                <div class="wd2-lib-tabs">
                  <StudioTabs
                    tabs={libTabs}
                    selected={libTab}
                    onSelect={(id) => (libTab = id)}
                    idPrefix="config-lib"
                  />
                </div>
                <div class="wd2-lib-panes">
                  <div
                    role="tabpanel"
                    id="config-lib-pane-themes"
                    aria-labelledby="config-lib-tab-themes"
                    hidden={libTab !== 'themes'}
                  >
                    <ThemesPanel dark={config.dark} bind:themes={themes} themeMeta={config.themeMeta} infoSlot={config.infoSlotId} />
                  </div>
                  <div
                    role="tabpanel"
                    id="config-lib-pane-backgrounds"
                    aria-labelledby="config-lib-tab-backgrounds"
                    hidden={libTab !== 'backgrounds'}
                  >
                    <BackgroundsPanel dark={config.dark} bind:backgrounds={backgrounds} trashTitle={config.trashTitle} />
                  </div>
                </div>
              </div>
            </div>
          </div>
          <footer class="wd2-foot">
            <span class="wd2-hint">{tx('studio_config_hint', 'Settings apply after save')}</span>
            <div class="wd2-actions">
              <button type="button" class="wd2-btn ghost" onclick={() => hide_modal()}>
                {tx('cancel', 'Cancel')}
              </button>
              <input type="submit" disabled={saving} value={text('save')} class="modal-button save-config {config.dark}" />
            </div>
          </footer>
        </form>
      </div>
    </div>
</ModalShell>
