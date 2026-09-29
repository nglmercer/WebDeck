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
import { normalizeHexColor, type SelectOption } from '../components/fields';
import { infoSlotId } from './svg';

/**
 * Port of `get_language` for the language dropdown: exact match, then
 * short-code request (`es` → `es_ES`), then same-language sibling
 * (`es_PE` → `es_ES`). Exported for tests.
 */
export function resolveLanguage(requested: string, langs: JsonObject[]): string {
  const req = requested.replace(/-/g, '_');
  const codes = langs.map((lang) => asString(lang['code']));
  const exact = codes.find((code) => code.toLowerCase() === req.toLowerCase());
  if (exact !== undefined) return exact;
  const lowered = req.toLowerCase();
  const shorts = codes.filter((code) => code.toLowerCase().startsWith(lowered)).sort();
  if (shorts.length > 0) return shorts[0] as string;
  const reqLang = lowered.split('_')[0] ?? '';
  if (reqLang !== '') {
    const siblings = codes
      .filter((code) => (code.toLowerCase().split('_')[0] ?? '') === reqLang)
      .sort();
    if (siblings.length > 0) return siblings[0] as string;
  }
  return 'en_US';
}

/**
 * Config data model (settings modal). Pure data in, markup out in
 * `Config.svelte` + `ThemesPanel`/`BackgroundsPanel` — every transform
 * below is the byte-identical logic the string `configView` used.
 */

export interface AudioSelectData {
  containerClass: string;
  id: string;
  name: string;
  label: string;
  options: SelectOption[];
}

/** Card metadata for one theme file (keyed by stripped filename). */
export interface ThemeMeta {
  icon: string;
  title: string;
  desc: string;
}

export interface ConfigData {
  dark: string;
  version: string;
  langOptions: SelectOption[];
  audioInput: AudioSelectData;
  audioOutput: AudioSelectData;
  spotifyUsername: string;
  spotifyClientId: string;
  spotifyClientSecret: string;
  obsHost: string;
  obsPort: string;
  obsPassword: string;
  namesColor: string;
  buttonsColor: string;
  height: string;
  width: string;
  portraitRotate: string;
  reloadTime: string;
  gpuOptions: SelectOption[];
  /** Ordered theme list (`//` prefix = disabled); owned by Config state. */
  themes: string[];
  /** Card metadata by stripped filename. */
  themeMeta: Record<string, ThemeMeta>;
  /** Ordered background list (`//` prefix = disabled); owned by Config state. */
  backgrounds: string[];
  infoSlotId: number;
  trashTitle: string;
  windowsStartup: boolean;
  autoUpdates: boolean;
  /** `invisible` extra class for exe-only switches (undefined in exe builds). */
  exeExtraClass: string | undefined;
  soundboardEnabled: boolean;
  earSoundboard: boolean;
  darkTheme: boolean;
  showNames: boolean;
  editButtonsColor: boolean;
  optimizedUsage: boolean;
  openInBrowser: boolean;
  showConsole: boolean;
  firewallBypass: boolean;
  fixSoundboard: boolean;
  devMode: boolean;
}

function audioSelectData(
  id: string,
  name: string,
  label: string,
  devices: string[],
  configured: string
): AudioSelectData {
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
  return {
    containerClass: id === 'mic_input_device' ? 'mic_input_device' : 'mic_output_device',
    id,
    name,
    label,
    options,
  };
}

function themeMeta(ctx: BootContext, themes: string[]): Record<string, ThemeMeta> {
  const parsed = asObject(ctx.parsed_themes);
  const meta: Record<string, ThemeMeta> = {};
  for (const theme of themes) {
    const entry = asObject(parsed[rep(theme, '//', '')]);
    meta[rep(theme, '//', '')] = {
      icon: asString(entry['theme-icon']),
      title: asString(entry['theme-name']),
      desc: asString(entry['theme-description']),
    };
  }
  return meta;
}

/** Settings modal data (index.jinja modal-container block). */
export function configData(ctx: BootContext): ConfigData {
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

  const gpuMethod = asString(settings['gpu_method']).toLowerCase();
  const gpuOptions: SelectOption[] = [
    { value: 'nvidia (NVML)', label: 'nvidia (NVML)', selected: gpuMethod === 'nvidia (nvml)' },
    {
      value: 'nvidia (NVML detailed)',
      label: 'nvidia (NVML detailed)',
      selected: gpuMethod === 'nvidia (nvml detailed)',
    },
    { value: 'AMD', label: `AMD (${text('doesnt_work')})`, selected: gpuMethod === 'amd' },
    { value: 'Intel', label: `Intel (${text('lmao')})`, selected: gpuMethod === 'intel' },
    { value: 'None', label: text('none'), selected: gpuMethod === 'none' },
  ];

  const configThemes = asArray(front['themes']).map((t) => asString(t));

  return {
    dark,
    version: asString(get(ctx.versions, 'versions', '0', 'version')),
    langOptions,
    audioInput: audioSelectData(
      'mic_input_device',
      'settings.soundboard.mic_input_device',
      text('input_device'),
      audioInput,
      asString(get(settings, 'soundboard', 'mic_input_device'))
    ),
    audioOutput: audioSelectData(
      'mic_output_device',
      'settings.soundboard.vbcable',
      text('output_device'),
      audioOutput,
      asString(get(settings, 'soundboard', 'vbcable'))
    ),
    spotifyUsername: asString(spotify['username']),
    spotifyClientId: asString(spotify['client_id']),
    spotifyClientSecret: asString(spotify['client_secret']),
    obsHost: asString(obs['host']),
    obsPort: asString(obs['port']),
    obsPassword: asString(obs['password']),
    namesColor: normalizeHexColor(asString(front['names_color'])),
    buttonsColor: normalizeHexColor(asString(front['buttons_color'])),
    height: asString(front['height']),
    width: asString(front['width']),
    portraitRotate: asString(front['portrait_rotate']),
    reloadTime: asString(front['computer_usage_reload_time']),
    gpuOptions,
    themes: configThemes,
    themeMeta: themeMeta(ctx, configThemes),
    infoSlotId: infoSlotId(dark),
    trashTitle: text('remove_background'),
    backgrounds: asArray(front['background']).map((b) => asString(b)),
    windowsStartup:
      'windows-startup' in settings ? asBool(settings['windows_startup']) : true,
    autoUpdates: 'auto-updates' in settings ? asBool(settings['auto_updates']) : true,
    exeExtraClass: ctx.is_exe ? undefined : 'invisible',
    soundboardEnabled: asBool(get(settings, 'soundboard', 'enabled')),
    earSoundboard: asBool(settings['ear_soundboard']),
    darkTheme: asBool(front['dark_theme']),
    showNames: asBool(front['show_names']),
    editButtonsColor: asBool(front['edit_buttons_color']),
    optimizedUsage: asBool(settings['optimized_usage_display']),
    openInBrowser: asBool(settings['open_settings_in_integrated_browser']),
    showConsole: asBool(settings['show_console']),
    firewallBypass: asBool(settings['automatic_firewall_bypass']),
    fixSoundboard: asBool(settings['fix_stop_soundboard']),
    devMode: asBool(settings['dev_mode']),
  };
}
