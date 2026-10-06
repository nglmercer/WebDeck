import { commandSchema, type Schema } from '../../lib/schema';
import type { Command, ButtonAction } from '../../lib/contracts';
import { localActionNames } from '../../lib/action-labels';
export const categories: Record<string, string> = {
  read: 'ui_readings_and_references',
  input: 'ui_keyboard_and_text',
  audio: 'ui_audio_and_media',
  window: 'ui_apps_and_windows',
  power: 'ui_power_and_system',
  script: 'ui_scripts',
  network: 'ui_network',
  plugin: 'ui_plugins',
  settings: 'ui_deck_settings',
};
export const names: Record<string, string> = {
  key: 'ui_keyboard_shortcut',
  write: 'ui_type_text',
  open: 'ui_open_app_or_url',
  play_pause: 'ui_play_pause',
  app_volume: 'ui_application_volume',
  fetch: 'ui_http_request',
  plugin: 'ui_plugin_action',
  button: 'ui_run_another_button',
  workflow: 'ui_workflow',
  debug: 'ui_debug_data',
};
export const descriptions: Record<string, string> = {
  key: 'ui_enter_a_chord_such_as_ctrl_c_desktop_permission_may_be_required',
  write: 'ui_type_text_on_the_host_computer_enable_send_to_press_enter_afterward',
  open: 'ui_open_a_url_application_or_file_on_the_host_computer',
  fetch: 'ui_send_an_http_request_from_the_host_network_permission_is_required',
  script: 'ui_run_a_javascript_script_with_the_permissions_granted_to_the_caller',
  button: 'ui_reference_another_configured_button_by_its_stable_id',
};

export type ActionOption = {
  id: string;
  type: string;
  command?: string;
  label: string;
  category: string;
  icon: string;
  description?: string;
  schema?: Schema;
};
export const actionIcons = {
  debug: 'bug',
  exit: 'exit',
  usage: 'chart',
  stop_sound: 'stop',
  play_sound: 'music',
  shutdown: 'power',
  reboot: 'refresh',
  sleep: 'moon',
  hibernate: 'hibernate',
  lock: 'lock',
  screensaver_settings: 'screen-settings',
  screensaver: 'screen-saver',
  key: 'keyboard',
  write: 'edit',
  copy: 'copy',
  paste: 'paste',
  cut: 'scissors',
  clipboard: 'clipboard',
  clear_clipboard: 'clipboard-clear',
  speech_recognition: 'voice',
  restart_desktop: 'desktop-restart',
  close_focused: 'window-close',
  color_picker: 'palette',
  kill: 'process-stop',
  restart: 'window-restart',
  foreground: 'window-focus',
  open: 'folder',
  volume: 'volume',
  app_volume: 'app-volume',
  mute: 'mute',
  play_pause: 'play',
  previous: 'previous',
  next: 'next',
  microphone: 'microphone',
  speakers: 'speaker',
  fetch: 'globe',
  script: 'code',
  shell: 'terminal',
  firewall: 'shield',
  obs: 'broadcast',
  spotify: 'spotify',
  plugin: 'puzzle',
  button: 'button-run',
  workflow: 'workflow',
  command: 'terminal',
  metric: 'chart',
  folder: 'folder',
  back: 'back',
  reload: 'refresh',
  fullscreen: 'fullscreen',
  settings: 'settings',
  edit: 'edit',
  none: 'circle-off',
} satisfies Record<Command['type'] | ButtonAction['type'], string>;
const icons: Record<string, string> = actionIcons;
export const actionOptions: ActionOption[] = [
  ...(commandSchema.oneOf ?? []).map((schema) => {
    const command = String(schema.properties?.type?.const ?? '');
    return {
      id: `command:${command}`,
      type: 'command',
      command,
      label: names[command] ?? command.replaceAll('_', ' ').replace(/^./, (c) => c.toUpperCase()),
      category: schema['x-capability'] ?? 'read',
      icon: icons[command] ?? 'settings',
      description: descriptions[command],
      schema,
    };
  }),
  ...[
    'folder',
    'back',
    'reload',
    'fullscreen',
    'settings',
    'metric',
    'edit',
    'none',
    'workflow',
    'script',
    'plugin',
  ].map((type) => ({
    id: type,
    type,
    label: localActionNames[type] ?? `ui_${type === 'plugin' ? 'plugin_action' : type}`,
    category:
      type === 'plugin'
        ? 'plugin'
        : type === 'script' || type === 'workflow'
          ? 'script'
          : 'settings',
    icon: icons[type] ?? 'grid',
  })),
];
