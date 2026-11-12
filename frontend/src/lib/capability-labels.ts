import type { TranslationKey } from './translations';
export const capabilityNames: Record<string, TranslationKey> = {
  read: 'permissions.viewDeckAndReadings',
  input: 'permissions.keyboardAndClipboard',
  audio: 'audio.andMedia',
  window: 'permissions.manageWindows',
  power: 'permissions.powerAndLock',
  script: 'permissions.runScripts',
  network: 'permissions.networkRequests',
  plugin: 'permissions.runPlugins',
  settings: 'permissions.editDeckSettings',
};
