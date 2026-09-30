import { saveConfig } from '../../api/config';
import { text } from '../../framework/i18n';
import { emitAppEvent } from '../../app/events';
import { refreshApp } from '../../app/refresh';
import { showAlert } from '../../components/dialog';
import { showError } from '../../app/toast';

export function settingsPayload(form: HTMLFormElement): Record<string, unknown> {
  const data: Record<string, unknown> = {};
  for (const input of form.querySelectorAll<HTMLInputElement | HTMLSelectElement>('input, select')) {
    const keys = input.name.split('.');
    if (!input.name || keys.some(key => ['__proto__', 'constructor', 'prototype'].includes(key))) continue;
    let target = data;
    for (const key of keys.slice(0,-1)) target = (target[key] ??= {}) as Record<string, unknown>;
    target[keys[keys.length-1]!] = input instanceof HTMLInputElement && input.type === 'checkbox'
      ? input.checked : input.id === 'language' ? input.value.toLowerCase() : input.value;
  }
  return data;
}
export async function submitSettings(form: HTMLFormElement): Promise<void> {
  try {
    const result = await saveConfig(settingsPayload(form));
    if (!result.success) { await showAlert(result.message || text('settings_save_error')); return; }
    emitAppEvent('save:completed', { flow: 'config' });
    await refreshApp();
    await showAlert(text('settings_save_success'));
  } catch (error) { showError(error instanceof Error ? error.message : text('settings_save_error')); }
}
