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
import { infoIcon } from './svg';

/** Port of `get_language` for the language dropdown (prefix match). */
function resolveLanguage(requested: string, langs: JsonObject[]): string {
  const lowered = requested.toLowerCase();
  for (const lang of langs) {
    const code = asString(lang['code']);
    if (code.toLowerCase().startsWith(lowered)) return code;
  }
  return 'en_US';
}

function switchRow(
  dark: string,
  cls: string,
  label: string,
  id: string,
  name: string,
  checked: boolean,
  extraCls = ''
): Html {
  return html`<div class="setting ${raw(cls)} ${raw(extraCls)}">
                  <p> ${label} </p>
                  <label for="${id}" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="${id}" name="${name}" ${checked ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                </div>`;
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
                      <svg class="choose-bg-delete-button" xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-trash" viewBox="0 0 16 16">
                        <title> ${text('remove_background')} </title>
                        <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6Z"></path>
                        <path d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1ZM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118ZM2.5 3h11V2h-11v1Z"></path>
                      </svg>
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
            <div class="background-color-input-container">
              <input type="color" class="background-color-input ${raw(dark)}" id="background-color-input" />
              <input type="text" id="background-color-hex" class="background-color-setting ${raw(dark)}" placeholder="${text('wallpaper_color')} (HEX)" />
            </div>
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
  ctx: BootContext,
  id: string,
  name: string,
  label: string,
  devices: string[],
  configured: string
): Html {
  const dark = ctx.dark_theme;
  const needle = configured.slice(configured.indexOf('(') + 1).toLowerCase();
  const options = devices.map((device) => {
    const display = !device.endsWith(')') && device.includes('(') ? device + '...' : device;
    // NOTE: empty needle matches everything upstream (Python `in`).
    const selected = display.toLowerCase().includes(needle);
    return html`<option value="${rep(display, '...', '')}" ${selected ? raw('selected') : raw('')}>
                          ${display}
                        </option>`;
  });
  return html`<div class="setting ${id === 'mic_input_device' ? 'mic_input_device' : 'mic_output_device'}">
                    <label for="${id}"> ${label} </label>
                    <select id="${id}" name="${name}">
                      ${join(options)}
                    </select>
                  </div>`;
}

function textInput(
  dark: string,
  cls: string,
  id: string,
  name: string,
  value: string,
  password: boolean,
  toggleId?: string
): Html {
  // NOTE: spotify-username/obs-HOST carry a duplicated class attribute upstream.
  const dupCls = id === 'spotify-username' || id === 'obs-HOST' ? html` class="${cls}"` : raw('');
  const field = html`<input class="${cls} ${raw(dark)}" type="${password ? 'password' : 'text'}"${dupCls} id="${id}" name="${name}" ${
    value.trim() !== '' ? html`value="${value}"` : raw('')
  } />`;
  if (!password || !toggleId) return field;
  return html`<div class="password-container">${field}<span id="${toggleId}" class="show-password" onclick="togglePasswordVisibility('${id}', '${toggleId}')"></span></div>`;
}

/** Settings modal (index.jinja modal-container block). */
export function configView(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  const config = ctx.config;
  const settings = asObject(get(config, 'settings'));
  const front = asObject(get(config, 'front'));

  const langs = ctx.langs.map((lang) => {
    const code = asString(lang['code']);
    const selected = resolveLanguage(asString(settings['language']), ctx.langs) === code;
    return html`<option value="${code}" ${selected ? raw('selected') : raw('')}>
                        ${asString(lang['native_name'])} (${code})
                      </option>`;
  });

  const audioInput = asArray(ctx.audio_devices['input']).map((d) => asString(d));
  const audioOutput = asArray(ctx.audio_devices['output']).map((d) => asString(d));
  const spotify = asObject(settings['spotify_api']);
  const obs = asObject(settings['obs']);

  const namesColor = asString(front['names_color']).trim();
  const defaultNamesColor =
    asString(front['names_color']) !== '' && namesColor !== ''
      ? namesColor.startsWith('#')
        ? `value="${namesColor}"`
        : `value="#${namesColor}"`
      : '';
  const buttonsColorCfg = asString(front['buttons_color']).trim();
  const defaultButtonsColor =
    asString(front['buttons_color']) !== '' && buttonsColorCfg !== ''
      ? buttonsColorCfg.startsWith('#')
        ? `value="${buttonsColorCfg}"`
        : `value="#${buttonsColorCfg}"`
      : '';

  const portraitRotate = asString(front['portrait_rotate']);
  const reloadTime = asString(front['computer_usage_reload_time']);
  const gpuMethod = asString(settings['gpu_method']).toLowerCase();
  const server = asString(settings['server']).toLowerCase();

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
            <svg class="config-modal ${raw(dark)}" xmlns="http://www.w3.org/2000/svg" width="19" height="19" fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16">
              <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z"/>
            </svg>
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
                <div class="setting language">
                  <label for="language"> ${text('language')} </label>
                  <select id="language" name="settings.language">
                    ${join(langs)}
                  </select>
                </div>
                ${switchRow(dark, 'windows-startup', text('config-windows_startup'), 'windows-startup', 'settings.windows_startup', 'windows-startup' in settings ? asBool(settings['windows_startup']) : true, ctx.is_exe ? '' : 'invisible')}
                ${switchRow(dark, 'auto-updates', text('auto_updates'), 'auto-updates', 'settings.auto_updates', 'auto-updates' in settings ? asBool(settings['auto_updates']) : true, ctx.is_exe ? '' : 'invisible')}
                <div class="setting soundboard ${raw(dark)}">
                  <div class="settings-title-info">
                    <label for="soundboard" style="margin-top: 3px;"> ${text('soundboard')} </label>
                    <a href="${text('link_soundboard')}" target="_blank" title="${text('soundboard_tutorial')}">
                        ${infoIcon(dark)}
                    </a>
                  </div>
                  ${switchRow(dark, 'toggle-soundboard', text('toggle_soundboard'), 'toggle_soundboard', 'settings.soundboard.enabled', asBool(get(settings, 'soundboard', 'enabled')))}
                  ${audioSelect(ctx, 'mic_input_device', 'settings.soundboard.mic_input_device', text('input_device'), audioInput, asString(get(settings, 'soundboard', 'mic_input_device')))}
                  ${audioSelect(ctx, 'mic_output_device', 'settings.soundboard.vbcable', text('output_device'), audioOutput, asString(get(settings, 'soundboard', 'vbcable')))}
                  ${switchRow(dark, 'ear-soundboard', text('ear_soundboard'), 'ear_soundboard', 'settings.ear_soundboard', asBool(settings['ear_soundboard']))}
                </div>
                <div class="setting spotify-api">
                  <div class="settings-title-info">
                    <label for="spotify" style="margin-top: 3px;"> ${text('spotify_api')} </label>
                    <a href="${text('link_spotify')}" target="_blank" title="${text('spotify_tutorial')}">
                        ${infoIcon(dark)}
                    </a>
                  </div>
                  <ul>
                    <li>
                      <label for="spotify-client_id"> ${text('username')} </label>
                      ${textInput(dark, 'spotify-setting', 'spotify-username', 'settings.spotify_api.username', asString(spotify['username']), false)}
                      <label for="spotify-client_id">Client ID</label>
                      ${textInput(dark, 'spotify-setting', 'spotify-client_id', 'settings.spotify_api.client_id', asString(spotify['client_id']), false)}
                      <label for="spotify-client_secret">Client Secret</label>
                      ${textInput(dark, 'spotify-setting', 'spotify-client_secret', 'settings.spotify_api.client_secret', asString(spotify['client_secret']), true, 'show-password-spotify')}
                    </li>
                  </ul>
                </div>
                <div class="setting obs-ws">
                  <div class="settings-title-info">
                    <label for="obs" style="margin-top: 3px;"> ${text('obs_studio')} </label>
                    <a href="${text('link_obs')}" target="_blank" title="${text('obs_tutorial')}">
                        ${infoIcon(dark)}
                    </a>
                  </div>
                  <ul>
                    <li>
                      <label for="obs-HOST"> ${text('host')} </label>
                      ${textInput(dark, 'obs-setting', 'obs-HOST', 'settings.obs.host', asString(obs['host']), false)}
                      <label for="obs-PORT"> ${text('port')} </label>
                      ${textInput(dark, 'obs-setting', 'obs-PORT', 'settings.obs.port', asString(obs['port']), false)}
                      <label for="obs-PASSWORD"> ${text('password')} </label>
                      ${textInput(dark, 'obs-setting', 'obs-PASSWORD', 'settings.obs.password', asString(obs['password']), true, 'show-password-obs')}
                    </li>
                  </ul>
                </div>
              </div>
              <div class="setting-category visuals ${raw(dark)}">
                <h1 class="config-title"> ${text('visuals')} </h1>
                <div class="setting gridsize">
                  <p> ${text('gridsize')} </p>
                  <div class="gridsize-container">
                    <div class="gridsize-height">
                      <label for="gridsize-height"> ${text('height')} </label>
                      <input min="1" pattern="[0-9]*"
                        oninput="this.value = this.value.replace(/[^0-9]/g, '');"
                        class="${raw(dark)}" type="number" id="gridsize-height" name="front.height" ${asString(front['height']).trim() !== '' ? html`value="${asString(front['height'])}"` : raw('')} />
                    </div>
                    <div class="gridsize-width">
                      <label for="gridsize-width"> ${text('width')} </label>
                      <input min="1" pattern="[0-9]*"
                        oninput="this.value = this.value.replace(/[^0-9]/g, '');"
                        class="${raw(dark)}" type="number" id="gridsize-width" name="front.width" ${asString(front['width']).trim() !== '' ? html`value="${asString(front['width'])}"` : raw('')} />
                    </div>
                  </div>
                </div>
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
                </div>
                <div class="setting ${raw(dark)}">
                  <p> ${text('dark_theme')} </p>
                  <label for="dark-theme" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="dark-theme" name="front.dark_theme" ${asBool(front['dark_theme']) ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                </div>
                <div class="setting portrait-rotate">
                  <label for="portrait-rotate"> ${text('portrait_rotate')} </label>
                  <div id="portrait-rotate-setting-container">
                    <input required class="${raw(dark)}" type="number" min="0" pattern="[0-9]*"
                      style="width: 75px;"
                      oninput="this.value = this.value.replaceAll(/[^0-9]/g, '');"
                      id="portrait-rotate" name="front.portrait_rotate"
                      ${portraitRotate.trim() !== '' ? html`value="${portraitRotate}"` : raw('value="90"')}
                    />
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
                </div>
                <div class="setting show-names">
                  <p> ${text('show_btn_names')} </p>
                  <label for="show-names" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="show-names" name="front.show_names" ${asBool(front['show_names']) ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                  <div class="names-color-input-container">
                    <input type="color" class="names-color-input ${raw(dark)}" id="names-color-input" ${raw(defaultNamesColor)} />
                    <input type="text" name="front.names_color" id="names-color-hex" class="names-color-setting ${raw(dark)}" placeholder="Button names color (HEX)" ${raw(defaultNamesColor)} />
                  </div>
                </div>
                <div class="setting edit-buttons-color">
                  <p> ${text('edit_btn_color')} </p>
                  <label for="edit_buttons_color" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="edit_buttons_color" name="front.edit_buttons_color" ${asBool(front['edit_buttons_color']) ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                  <div class="buttons-color-input-container">
                    <input type="color" class="buttons-color-input ${raw(dark)}" id="buttons-color-input" ${raw(defaultButtonsColor)} />
                    <input type="text" name="front.buttons_color" id="buttons-color-hex" class="buttons-color-setting ${raw(dark)}" placeholder="Button default color (HEX)" ${raw(defaultButtonsColor)} />
                  </div>
                </div>
              </div>
              <div class="setting-category experimental ${raw(dark)}">
                <h1 class="config-title"> ${text('experimental')} </h1>
                <div class="setting usage-reload-time">
                  <label for="usage-reload-time"> ${text('usage_btn_reload_time')} </label>
                  <input required class="${raw(dark)}" type="number" min="0" pattern="[0-9]*"
                    oninput="this.value = this.value.replaceAll(/[^0-9]/g, '');"
                    id="usage-reload-time" name="front.computer_usage_reload_time"
                    ${reloadTime.trim() !== '' ? html`value="${reloadTime}"` : raw('value="3000"')}
                  />
                </div>
                <div class="setting gpu_method">
                  <label for="gpu_method"> ${text('gpu_usage_method')} </label>
                  <select id="gpu_method" name="settings.gpu_method">
                    <option value="nvidia (pynvml)" ${gpuMethod === 'nvidia (pynvml)' ? raw('selected') : raw('')}>nvidia (pynvml)</option>
                    <option value="nvidia (GPUtil)" ${gpuMethod === 'nvidia (gputil)' ? raw('selected') : raw('')}>nvidia (GPUtil)</option>
                    <option value="AMD" ${gpuMethod === 'amd' ? raw('selected') : raw('')}>AMD (${text('doesnt_work')})</option>
                    <option value="Intel" ${gpuMethod === 'intel' ? raw('selected') : raw('')}>Intel (${text('lmao')})</option>
                    <option value="None" ${gpuMethod === 'none' ? raw('selected') : raw('')}> ${text('none')} </option>
                  </select>
                </div>
                ${switchRow(dark, 'optimized-usage-display', text('optimized_usage_display'), 'optimized_usage_display', 'settings.optimized_usage_display', asBool(settings['optimized_usage_display']))}
                ${switchRow(dark, 'open-settings-in-integrated-browser', text('open_settings_in_integrated_browser'), 'open_settings_in_integrated_browser', 'settings.open_settings_in_integrated_browser', asBool(settings['open_settings_in_integrated_browser']))}
                <div class="setting show-console">
                  <p> ${text('show_console')} </p>
                  <label for="show_console" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="show_console" name="settings.show_console" ${asBool(settings['show_console']) ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                </div>
                <div class="setting automatic-firewall-bypass">
                  <button type="button" onclick="send_data('/bypass-windows-firewall')" class="button" id="authorize_windows_firewall">
                    ${text('authorize_windows_firewall')}
                  </button>
                  <p> ${text('automatic_firewall_bypass')} </p>
                  <label for="automatic_firewall_bypass" class="switch">
                    <input class="${raw(dark)}" type="checkbox" id="automatic_firewall_bypass" name="settings.automatic_firewall_bypass" ${asBool(settings['automatic_firewall_bypass']) ? raw('checked') : raw('')} />
                    <span class="slider round"></span>
                  </label>
                </div>
                ${switchRow(dark, 'fix-stop-soundboard', text('fix_stop_soundboard'), 'fix_stop_soundboard', 'settings.fix_stop_soundboard', asBool(settings['fix_stop_soundboard']))}
                <div class="setting server">
                  <label for="server"> ${text('server')} </label>
                  <select id="server" name="settings.server">
                    <option value="flask" ${server === 'flask' ? raw('selected') : raw('')}>Flask (app.run)</option>
                    <option value="werkzeug" ${server === 'werkzeug' ? raw('selected') : raw('')}>Werkzeug (make_server)</option>
                  </select>
                </div>
                ${switchRow(dark, 'flask-debug', text('flask_debug'), 'flask_debug', 'settings.flask_debug', asBool(settings['flask_debug']))}
                ${switchRow(dark, 'flask-reloader', text('flask_reloader'), 'flask_reloader', 'settings.flask_reloader', asBool(settings['flask_reloader']))}
                ${switchRow(dark, 'dev-mode', text('dev_mode'), 'dev_mode', 'settings.dev_mode', asBool(settings['dev_mode']))}
              </div>
              <input type="submit" value="${text('save')}" class="modal-button save-config ${raw(dark)}" />
            </form>
          </div>
        </div>
      </div>
    </div>`;
}
