import type { TranslationKey } from '../../lib/translations';
import { commandSchema, type Schema } from '../../lib/schema';
import type { Command, ButtonAction, PluginManifest } from '../../lib/contracts';
import { localActionNames } from '../../lib/action-labels';
export const categories: Record<string, TranslationKey> = {
  read: 'actions.readingsAndReferences',
  input: 'actions.keyboardAndText',
  audio: 'audio.andMedia',
  window: 'actions.appsAndWindows',
  power: 'actions.powerAndSystem',
  script: 'actions.scripts',
  network: 'actions.network',
  plugin: 'actions.plugins',
  settings: 'actions.deckSettings',
};
export const names: Record<string, TranslationKey> = {
  debug: 'actions.debugData',
  exit: 'commands.exit',
  usage: 'commands.usage',
  stop_sound: 'commands.stopSound',
  play_sound: 'commands.playSound',
  shutdown: 'commands.shutdown',
  reboot: 'commands.reboot',
  sleep: 'commands.sleep',
  hibernate: 'commands.hibernate',
  lock: 'commands.lock',
  screensaver_settings: 'commands.screensaverSettings',
  screensaver: 'commands.screensaver',
  key: 'actions.keyboardShortcut',
  write: 'actions.typeText',
  copy: 'commands.copy',
  paste: 'commands.paste',
  cut: 'commands.cut',
  clipboard: 'commands.clipboard',
  clear_clipboard: 'commands.clearClipboard',
  speech_recognition: 'commands.speechRecognition',
  restart_desktop: 'commands.restartDesktop',
  close_focused: 'commands.closeFocused',
  color_picker: 'commands.colorPicker',
  kill: 'commands.kill',
  restart: 'commands.restart',
  foreground: 'commands.foreground',
  open: 'actions.openAppOrUrl',
  volume: 'audio.volume',
  app_volume: 'actions.applicationVolume',
  mute: 'commands.mute',
  play_pause: 'actions.playPause',
  previous: 'common.previous',
  next: 'common.next',
  microphone: 'audio.microphone',
  speakers: 'commands.speakers',
  fetch: 'actions.httpRequest',
  script: 'commands.script',
  shell: 'commands.shell',
  firewall: 'commands.firewall',
  obs: 'integrations.obs',
  spotify: 'integrations.spotify',
  plugin: 'editor.pluginAction',
  button: 'actions.runAnotherButton',
  workflow: 'actions.workflow',
};
export const descriptions: Record<string, TranslationKey> = {
  key: 'actions.help.key',
  write: 'actions.help.write',
  open: 'actions.help.open',
  fetch: 'actions.help.fetch',
  script: 'actions.help.script',
  button: 'actions.help.button',
};

export type ActionOption = {
  id: string;
  type: string;
  command?: string;
  label: string;
  /** Plain text from a plugin manifest, shown as written rather than looked up as a key. */
  detail?: string;
  plugin?: { pluginId: string; actionId: string };
  category: string;
  icon: string;
  description?: TranslationKey | undefined;
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
    label: localActionNames[type] ?? names[type] ?? type,
    category:
      type === 'plugin'
        ? 'plugin'
        : type === 'script' || type === 'workflow'
          ? 'script'
          : 'settings',
    icon: icons[type] ?? 'grid',
  })),
];

/** The picker id of a plugin action, which is also how the editor recognises the action it runs. */
export function pluginOptionId(pluginId: string, actionId: string): string {
  return `plugin:${pluginId}:${actionId}`;
}

/** The argument fields of one plugin action, as the editor's form schema. */
export function pluginArgumentsSchema(plugin: PluginManifest, actionId: string): Schema {
  const action = plugin.actions.find((candidate) => candidate.id === actionId);
  return {
    type: 'object',
    additionalProperties: false,
    properties: Object.fromEntries(
      // A declared list of choices becomes a select in the form; the generic fields render it.
      Object.entries(action?.arguments ?? {}).map(([name, argument]) => [
        name,
        argument.enum ? { type: argument.type, enum: argument.enum } : { type: argument.type },
      ]),
    ),
  };
}

/** One picker entry per action of every loaded plugin, so each action is found by its own name. */
export function pluginActionOptions(plugins: PluginManifest[]): ActionOption[] {
  return plugins.flatMap((plugin) =>
    plugin.actions.map((action) => ({
      id: pluginOptionId(plugin.id, action.id),
      type: 'command',
      command: 'plugin',
      label: action.label,
      detail: `${plugin.id} · ${plugin.version}`,
      category: 'plugin',
      icon: icons.plugin ?? 'puzzle',
      plugin: { pluginId: plugin.id, actionId: action.id },
      schema: pluginArgumentsSchema(plugin, action.id),
    })),
  );
}

/** The picker's entries: built-in actions, with the generic plugin entries replaced by one per plugin action. */
export function pickerActionOptions(plugins: PluginManifest[]): ActionOption[] {
  return [
    ...actionOptions.filter((option) => option.command !== 'plugin' && option.type !== 'plugin'),
    ...pluginActionOptions(plugins),
  ];
}
