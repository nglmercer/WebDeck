import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import {
  asArray,
  asBool,
  asObject,
  asString,
  get,
  rep,
  type BootContext,
  type JsonObject,
} from '../framework/types';
import { collapseSection } from '../components/collapse';
import {
  colorField,
  normalizeHexColor,
  numberField,
  selectField,
  switchField,
  textField,
  type SelectOption,
} from '../components/fields';
import { infoIcon } from './svg';
import { modalCloseIcon, trashIcon } from '../components/icons';

/** Port of `get_language` for the language dropdown (prefix match). */
function resolveLanguage(requested: string, langs: JsonObject[]): string {
  const lowered = requested.toLowerCase();
  for (const lang of langs) {
    const code = asString(lang['code']);
    if (code.toLowerCase().startsWith(lowered)) return code;
  }
  return 'en_US';
}

function themesPanel(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  const configThemes = asArray(get(ctx.config, 'front', 'themes')).map((t) => asString(t));
  const parsed = asObject(ctx.parsed_themes);

  const disabled = configThemes
    .filter((theme) => theme.startsWith('//'))
    .map((theme) => {
      const name = rep(theme, '//', '');
      const icon = asString(asObject(parsed[name])['theme-icon']);
      const title = asString(asObject(parsed[name])['theme-name']);
      const desc = asString(asObject(parsed[name])['theme-description']);
      return html`<div class="theme-container" filename="${theme}">
                      <span class="enable-theme-hitbox"></span>
                      <span class="enable-theme invisible"></span>
                      <div class="arrows-container invisible">
                        <span class="arrow-up-hitbox"></span>
                        <span class="arrow-up"></span>
                        <span class="arrow-down-hitbox"></span>
                        <span class="arrow-down"></span>
                      </div>
                      <div class="theme-icon-container">
                        ${icon !== '' ? html`<img class="theme-icon" src="${icon}"/>` : raw('')}
                      </div>
                      <div class="theme-texts">
                        <div class="theme-title">
                          ${title}
                        </div>
                        <div class="theme-desc">
                          ${desc}
                        </div>
                      </div>
                    </div>`;
    });

  // NOTE: the boot config already includes style.css once (server pre-push,
  // mirroring the template-time append), so no append is needed here.
  const enabled = configThemes
    .filter((theme) => !theme.startsWith('//'))
    .map((theme) => {
      const isDefault = theme === 'static/css/style.css' ? 'defaulttheme' : '';
      const icon = asString(asObject(parsed[theme])['theme-icon']);
      const title = asString(asObject(parsed[theme])['theme-name']);
      const desc = asString(asObject(parsed[theme])['theme-description']);
      return html`<div class="theme-container" filename="${theme}" ${raw(isDefault)}>
                      ${
                        theme !== 'static/css/style.css'
                          ? html`<span class="disable-theme-hitbox"></span>
                        <span class="disable-theme invisible"></span>
                        <div class="arrows-container invisible">
                          <span class="arrow-up-hitbox"></span>
                          <span class="arrow-up"></span>
                          <span class="arrow-down-hitbox"></span>
                          <span class="arrow-down"></span>
                        </div>`
                          : raw('')
                      }
                      <div class="theme-icon-container">
                        ${icon !== '' ? html`<img class="theme-icon" src="${icon}"/>` : raw('')}
                      </div>
                      <div class="theme-texts">
                        <div class="theme-title">
                          ${title}
                        </div>
                        <div class="theme-desc">
                          ${desc}
                        </div>
                      </div>
                    </div>`;
    });

  // NOTE: the lone "<" below mirrors an upstream typo (renders as text).
  return html`<div class="config-container choose-themes ${raw(dark)}" id="choose-themes" style="display: none;">
            <
            <button type="button" id="setting-themes-back" class="button ${raw(dark)}">
              ${text('back')}
            </button>
            <div id="themes-tutorial-container">
              <a href="${text('link_themes')}" target="_blank" title="${text('themes_tutorial')}">
                  ${infoIcon(dark)}
              </a>
              <button type="button" onclick="send_data('/openfolder /.config/themes')" class="button" id="open-themes-folder">
                ${text('open_themes_folder')}
              </button>
            </div>
            <h1 class="config-title"> ${text('themes_menu_title')} </h1>
            <div class="editorStyle-bar ${raw(dark)}"></div>
            <div id="choose-themes-titles">
              <h2 class="disabled-themes-title"> ${text('disabled_themes')} </h2>
              <h2 class="enabled-themes-title"> ${text('enabled_themes')} </h2>
            </div>
            <div id="choose-themes-container">
              <div id="disabled-themes">
                ${join(disabled)}
              </div>
              <div id="enabled-themes">
                ${join(enabled)}
              </div>
            </div>
          </div>`;
}

function backgroundsPanel(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  const backgrounds = asArray(get(ctx.config, 'front', 'background')).map((b) => asString(b));

  const elements = backgrounds.map((bg) => {
    const activateCls = bg.startsWith('//')
      ? 'choose-bg-activate-button'
      : 'choose-bg-activate-button choose-bg-activate-button-checked';
    const buttons = html`<div class="choose-bg-buttons">
                      <div class="${raw(activateCls)}"></div>
                      ${trashIcon(text('remove_background'))}
                    </div>`;
    const stripped = rep(bg, '//', '');
    if (stripped.startsWith('rgb(') || stripped.startsWith('#')) {
      return html`<div class="choose-bg-element choose-bg-element-color choose-bg-element-pageload ${raw(dark)}"
                    background="${bg}"
                    background_color_text="${text('background_color')}"
                    style="background-color: ${stripped};"
                  >
                    ${text('background_color')} : ${stripped}
                    ${buttons}
                  </div>`;
    }
    if (bg.endsWith('.mp4')) {
      const src = '.config/user_uploads/' + rep(stripped, '**uploaded/', '');
      return html`<div class="choose-bg-element choose-bg-element-image" background="${bg}">
                    <div class="video-container choose-bg-pseudo-element">
                      <video autoplay muted loop class="blurred-video">
                        <source src="${src}" type="video/mp4" />
                        Your browser does not support the video tag...
                      </video>
                    </div>
                    <div class="video-container">
                      <video autoplay muted loop>
                        <source src="${src}" type="video/mp4" />
                        Your browser does not support the video tag...
                      </video>
                    </div>
                    ${buttons}
                  </div>`;
    }
    const file = rep(stripped, '**uploaded/', '');
    return html`<div class="choose-bg-element choose-bg-element-image" background="${bg}">
                    <div class="choose-bg-pseudo-element" style='background-image: url("${raw(`.config/user_uploads/${rep(file, "'", '&#39;')}`)}")'></div>
                    <img src="${'.config/user_uploads/' + file}" />
                    ${buttons}
                  </div>`;
  });

  // NOTE: the lone "<" below mirrors an upstream typo (renders as text).
  return html`<div class="config-container choose-background ${raw(dark)}" id="choose-background" style="display: none;">
            <
            <button type="button" id="setting-background-back" class="button ${raw(dark)}">
              ${text('back')}
            </button>
            <h1 class="config-title"> ${text('random_bg_menu_title')} </h1>
            ${colorField({
              dark,
              containerClass: 'background-color-input-container',
              colorClass: 'background-color-input',
              colorId: 'background-color-input',
              hexClass: 'background-color-setting',
              hexId: 'background-color-hex',
              placeholder: `${text('wallpaper_color')} (HEX)`,
              value: '',
            })}
            <div id="create-bg-choices">
              <button class="${raw(dark)}" id="create-color-bg"> ${text('add_background_color')} </button>
              ${text('or')}
              <input type="file" id="create-image-bg" class="${raw(dark)}" accept="image/jpeg, image/png, image/gif, video/mp4" />
            </div>
            <div class="editorStyle-bar ${raw(dark)}"></div>
            <div id="choose-backgrounds-container">
              ${join(elements)}
            </div>
          </div>`;
}

function audioSelect(
  id: string,
  name: string,
  label: string,
  devices: string[],
  configured: string
): Html {
  const needle = configured.slice(configured.indexOf('(') + 1).toLowerCase();
  const options: SelectOption[] = devices.map((device) => {
    const display = !device.endsWith(')') && device.includes('(') ? device + '...' : device;
    // NOTE: empty needle matches everything upstream (Python `in`).
    return {
      value: rep(display, '...', ''),
      label: display,
      selected: display.toLowerCase().includes(needle),
    };
  });
  return selectField({
    containerClass: id === 'mic_input_device' ? 'mic_input_device' : 'mic_output_device',
    id,
    name,
    label,
    options,
  });
}

/** Settings modal (index.jinja modal-container block). */
export function configView(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  const config = ctx.config;
  const settings = asObject(get(config, 'settings'));
  const front = asObject(get(config, 'front'));

  const langOptions: SelectOption[] = ctx.langs.map((lang) => {
    const code = asString(lang['code']);
    return {
      value: code,
      label: `${asString(lang['native_name'])} (${code})`,
      selected: resolveLanguage(asString(settings['language']), ctx.langs) === code,
    };
  });

  const audioInput = asArray(ctx.audio_devices['input']).map((d) => asString(d));
  const audioOutput = asArray(ctx.audio_devices['output']).map((d) => asString(d));
  const spotify = asObject(settings['spotify_api']);
  const obs = asObject(settings['obs']);

  const namesColor = normalizeHexColor(asString(front['names_color']));
  const buttonsColor = normalizeHexColor(asString(front['buttons_color']));

  const portraitRotate = asString(front['portrait_rotate']);
  const reloadTime = asString(front['computer_usage_reload_time']);
  const gpuMethod = asString(settings['gpu_method']).toLowerCase();
  const server = asString(settings['server']).toLowerCase();

  const gpuOptions: SelectOption[] = [
    { value: 'nvidia (pynvml)', label: 'nvidia (pynvml)', selected: gpuMethod === 'nvidia (pynvml)' },
    { value: 'nvidia (GPUtil)', label: 'nvidia (GPUtil)', selected: gpuMethod === 'nvidia (gputil)' },
    { value: 'AMD', label: `AMD (${text('doesnt_work')})`, selected: gpuMethod === 'amd' },
    { value: 'Intel', label: `Intel (${text('lmao')})`, selected: gpuMethod === 'intel' },
    { value: 'None', label: text('none'), selected: gpuMethod === 'none' },
  ];

  const serverOptions: SelectOption[] = [
    { value: 'flask', label: 'Flask (app.run)', selected: server === 'flask' },
    { value: 'werkzeug', label: 'Werkzeug (make_server)', selected: server === 'werkzeug' },
  ];

  // NOTE: upstream renders this unescaped (invalid HTML, breaks theme
  // persistence); emit valid JSON so the feature actually works.
  const themesRepr = JSON.stringify(asArray(front['themes']).map((t) => asString(t)));
  const bgRepr =
    '[' + asArray(front['background']).map((b) => `'${asString(b)}'`).join(', ') + ']';

  return html`
    <div class="modal-container ${raw(dark)}" id="modal-container">
      <div class="modal-content ${raw(dark)}">
        <div class="modal-header bold">
          <h1 class="config-modal"> ${text('configuration')} </h1>
          <div class="modal-close">
            ${modalCloseIcon('config-modal', raw(dark))}
          </div>
        </div>
        <div style="display: flex; flex-direction: column;">
          <a class="${raw(dark)}" id="version" href="https://github.com/Lenochxd/WebDeck/releases/tag/v${asString(
            get(ctx.versions, 'versions', '0', 'version')
          )}" target="_blank" title="${text('see_patchnotes')}" alt="${text('see_patchnotes')}">
            v${asString(get(ctx.versions, 'versions', '0', 'version'))}
          </a>
          <a class="${raw(dark)}" id="version" href="https://github.com/Lenochxd/WebDeck/issues/new/choose" target="_blank" alt="${text('github_issue')}" style="margin-top: -4px;">
            ${text('github_issue')}
          </a>
        </div>
        <div class="modal-main">
          ${themesPanel(ctx)}
          ${backgroundsPanel(ctx)}
          <div class="config-container ${raw(dark)}" id="config-container" style="display: block;">
            <button class="button modal-button" id="editorButton"> [Q] ${text('enter_editor_mode')} </button>
            <form id="config-form" class="config-form">
              <div class="setting-category settings ${raw(dark)}">
                <h1 class="config-title"> ${text('settings')} </h1>
                ${collapseSection({
                  id: 'settings-general',
                  title: text('settings_group_general'),
                  open: true,
                  body: html`
                ${selectField({ containerClass: 'language', id: 'language', name: 'settings.language', label: text('language'), options: langOptions })}
                ${switchField({ dark, containerClass: 'windows-startup', label: text('config-windows_startup'), id: 'windows-startup', name: 'settings.windows_startup', checked: 'windows-startup' in settings ? asBool(settings['windows_startup']) : true, extraClass: ctx.is_exe ? undefined : 'invisible' })}
                ${switchField({ dark, containerClass: 'auto-updates', label: text('auto_updates'), id: 'auto-updates', name: 'settings.auto_updates', checked: 'auto-updates' in settings ? asBool(settings['auto_updates']) : true, extraClass: ctx.is_exe ? undefined : 'invisible' })}`,
                })}
                ${collapseSection({
                  id: 'settings-soundboard',
                  title: text('soundboard'),
                  info: { href: text('link_soundboard'), title: text('soundboard_tutorial'), icon: infoIcon(dark) },
                  body: html`
                ${switchField({ dark, containerClass: 'toggle-soundboard', label: text('toggle_soundboard'), id: 'toggle_soundboard', name: 'settings.soundboard.enabled', checked: asBool(get(settings, 'soundboard', 'enabled')) })}
                ${audioSelect('mic_input_device', 'settings.soundboard.mic_input_device', text('input_device'), audioInput, asString(get(settings, 'soundboard', 'mic_input_device')))}
                ${audioSelect('mic_output_device', 'settings.soundboard.vbcable', text('output_device'), audioOutput, asString(get(settings, 'soundboard', 'vbcable')))}
                ${switchField({ dark, containerClass: 'ear-soundboard', label: text('ear_soundboard'), id: 'ear_soundboard', name: 'settings.ear_soundboard', checked: asBool(settings['ear_soundboard']) })}`,
                })}
                ${collapseSection({
                  id: 'settings-spotify',
                  title: text('spotify_api'),
                  info: { href: text('link_spotify'), title: text('spotify_tutorial'), icon: infoIcon(dark) },
                  body: html`
                ${textField({ dark, cls: 'spotify-setting', label: text('username'), labelFor: 'spotify-username', id: 'spotify-username', name: 'settings.spotify_api.username', value: asString(spotify['username']) })}
                ${textField({ dark, cls: 'spotify-setting', label: 'Client ID', id: 'spotify-client_id', name: 'settings.spotify_api.client_id', value: asString(spotify['client_id']) })}
                ${textField({ dark, cls: 'spotify-setting', label: 'Client Secret', id: 'spotify-client_secret', name: 'settings.spotify_api.client_secret', value: asString(spotify['client_secret']), password: true, toggleId: 'show-password-spotify' })}`,
                })}
                ${collapseSection({
                  id: 'settings-obs',
                  title: text('obs_studio'),
                  info: { href: text('link_obs'), title: text('obs_tutorial'), icon: infoIcon(dark) },
                  body: html`
                ${textField({ dark, cls: 'obs-setting', label: text('host'), id: 'obs-HOST', name: 'settings.obs.host', value: asString(obs['host']) })}
                ${textField({ dark, cls: 'obs-setting', label: text('port'), id: 'obs-PORT', name: 'settings.obs.port', value: asString(obs['port']) })}
                ${textField({ dark, cls: 'obs-setting', label: text('password'), id: 'obs-PASSWORD', name: 'settings.obs.password', value: asString(obs['password']), password: true, toggleId: 'show-password-obs' })}`,
                })}
              </div>
              <div class="setting-category visuals ${raw(dark)}">
                <h1 class="config-title"> ${text('visuals')} </h1>
                ${collapseSection({
                  id: 'visuals-grid',
                  title: text('settings_group_grid_layout'),
                  open: true,
                  body: html`
                <div class="setting gridsize">
                  <p> ${text('gridsize')} </p>
                  <div class="gridsize-container">
                    <div class="gridsize-height">
                      <label for="gridsize-height"> ${text('height')} </label>
                      ${numberField({ dark, id: 'gridsize-height', name: 'front.height', value: asString(front['height']), min: '1' })}
                    </div>
                    <div class="gridsize-width">
                      <label for="gridsize-width"> ${text('width')} </label>
                      ${numberField({ dark, id: 'gridsize-width', name: 'front.width', value: asString(front['width']), min: '1' })}
                    </div>
                  </div>
                </div>
                <div class="setting portrait-rotate">
                  <label for="portrait-rotate"> ${text('portrait_rotate')} </label>
                  <div id="portrait-rotate-setting-container">
                    ${numberField({ dark, id: 'portrait-rotate', name: 'front.portrait_rotate', value: portraitRotate, defaultValue: '90', min: '0', required: true, style: 'width: 75px;' })}
                    <div class="deg">&deg;</div>
                    <button type="button" onclick="document.getElementById('portrait-rotate').value = '270'" class="button">
                      270&deg;
                    </button>
                    <button type="button" onclick="document.getElementById('portrait-rotate').value = '90'" class="button">
                      90&deg;
                    </button>
                    <button type="button" onclick="document.getElementById('portrait-rotate').value = '0'" class="button">
                      0&deg;
                    </button>
                  </div>
                </div>`,
                })}
                ${collapseSection({
                  id: 'visuals-theme',
                  title: text('settings_group_theme_background'),
                  body: html`
                <div class="setting themes">
                  <label for="themes"> ${text('themes')} </label>
                  <button type="button" id="setting-themes" class="${raw(dark)}">
                    ${text('open_theme_menu')}
                  </button>
                  <input type="text" name="front.themes" id="choose-themes-handler" class="invisible" value="${themesRepr}" />
                </div>
                <div class="setting background">
                  <label for="background"> ${text('backgrounds')} </label>
                  <button type="button" id="setting-background" class="${raw(dark)}">
                    ${text('open_background_image_menu')}
                  </button>
                  <input type="text" name="front.background" id="choose-background-handler" class="invisible" value="${bgRepr}" />
                </div>`,
                })}
                ${collapseSection({
                  id: 'visuals-appearance',
                  title: text('settings_group_appearance'),
                  body: html`
                ${switchField({ dark, containerClass: dark, label: text('dark_theme'), id: 'dark-theme', name: 'front.dark_theme', checked: asBool(front['dark_theme']) })}
                ${switchField({ dark, containerClass: 'show-names', label: text('show_btn_names'), id: 'show-names', name: 'front.show_names', checked: asBool(front['show_names']) })}
                ${colorField({ dark, containerClass: 'names-color-input-container', colorClass: 'names-color-input', colorId: 'names-color-input', hexClass: 'names-color-setting', hexId: 'names-color-hex', hexName: 'front.names_color', placeholder: 'Button names color (HEX)', value: namesColor })}
                ${switchField({ dark, containerClass: 'edit-buttons-color', label: text('edit_btn_color'), id: 'edit_buttons_color', name: 'front.edit_buttons_color', checked: asBool(front['edit_buttons_color']) })}
                ${colorField({ dark, containerClass: 'buttons-color-input-container', colorClass: 'buttons-color-input', colorId: 'buttons-color-input', hexClass: 'buttons-color-setting', hexId: 'buttons-color-hex', hexName: 'front.buttons_color', placeholder: 'Button default color (HEX)', value: buttonsColor })}`,
                })}
              </div>
              <div class="setting-category experimental ${raw(dark)}">
                <h1 class="config-title"> ${text('experimental')} </h1>
                ${collapseSection({
                  id: 'experimental-usage',
                  title: text('settings_group_usage'),
                  open: true,
                  body: html`
                <div class="setting usage-reload-time">
                  ${numberField({ dark, id: 'usage-reload-time', name: 'front.computer_usage_reload_time', label: text('usage_btn_reload_time'), value: reloadTime, defaultValue: '3000', min: '0', required: true })}
                </div>
                ${selectField({ containerClass: 'gpu_method', id: 'gpu_method', name: 'settings.gpu_method', label: text('gpu_usage_method'), options: gpuOptions })}
                ${switchField({ dark, containerClass: 'optimized-usage-display', label: text('optimized_usage_display'), id: 'optimized_usage_display', name: 'settings.optimized_usage_display', checked: asBool(settings['optimized_usage_display']) })}`,
                })}
                ${collapseSection({
                  id: 'experimental-advanced',
                  title: text('settings_group_advanced'),
                  body: html`
                ${switchField({ dark, containerClass: 'open-settings-in-integrated-browser', label: text('open_settings_in_integrated_browser'), id: 'open_settings_in_integrated_browser', name: 'settings.open_settings_in_integrated_browser', checked: asBool(settings['open_settings_in_integrated_browser']) })}
                ${switchField({ dark, containerClass: 'show-console', label: text('show_console'), id: 'show_console', name: 'settings.show_console', checked: asBool(settings['show_console']) })}
                <div class="setting automatic-firewall-bypass">
                  <button type="button" onclick="send_data('/bypass-windows-firewall')" class="button" id="authorize_windows_firewall">
                    ${text('authorize_windows_firewall')}
                  </button>
                </div>
                ${switchField({ dark, containerClass: 'automatic-firewall-bypass-toggle', label: text('automatic_firewall_bypass'), id: 'automatic_firewall_bypass', name: 'settings.automatic_firewall_bypass', checked: asBool(settings['automatic_firewall_bypass']) })}
                ${switchField({ dark, containerClass: 'fix-stop-soundboard', label: text('fix_stop_soundboard'), id: 'fix_stop_soundboard', name: 'settings.fix_stop_soundboard', checked: asBool(settings['fix_stop_soundboard']) })}
                ${selectField({ containerClass: 'server', id: 'server', name: 'settings.server', label: text('server'), options: serverOptions })}
                ${switchField({ dark, containerClass: 'flask-debug', label: text('flask_debug'), id: 'flask_debug', name: 'settings.flask_debug', checked: asBool(settings['flask_debug']) })}
                ${switchField({ dark, containerClass: 'flask-reloader', label: text('flask_reloader'), id: 'flask_reloader', name: 'settings.flask_reloader', checked: asBool(settings['flask_reloader']) })}
                ${switchField({ dark, containerClass: 'dev-mode', label: text('dev_mode'), id: 'dev_mode', name: 'settings.dev_mode', checked: asBool(settings['dev_mode']) })}`,
                })}
              </div>
              <input type="submit" value="${text('save')}" class="modal-button save-config ${raw(dark)}" />
            </form>
          </div>
        </div>
      </div>
    </div>`;
}
