<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  import { onDestroy, onMount } from 'svelte';
  import {
    keys,
    keyLabel,
    normalizeKey,
    validateChord,
    captureChord,
    platformSupport,
  } from './keys';
  let { value, onchange }: { value: string[]; onchange: (keys: string[]) => void } = $props();
  import { request } from '../../lib/api/http';
  import type { KeyboardCapabilities } from '../../lib/contracts';
  let capabilities = $state<KeyboardCapabilities | undefined>();
  onMount(() => {
    const controller = new AbortController();
    request<KeyboardCapabilities>('keyboard/capabilities', 'KeyboardCapabilities', {
      signal: controller.signal,
    })
      .then((value) => {
        capabilities = value;
      })
      .catch(() => {});
    return () => controller.abort();
  });
  const extendedAvailable = $derived(
    capabilities?.platform !== undefined && capabilities.platform !== 'unsupported',
  );
  let cleared = $state(false);
  const selectedKeys = $derived(cleared ? [] : value);
  let search = $state('');
  let category = $state('function');
  let recording = $state(false);
  let capture: HTMLButtonElement;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let message = $state('');
  let advanced = $state('');
  let showExtended = $state(false);
  const categories = [...new Set(keys.map((k) => k.category))];
  const visible = $derived(
    keys.filter(
      (k) =>
        (!search
          ? k.category === category
          : `${k.label} ${k.id} ${k.category}`.toLowerCase().includes(search.toLowerCase())) &&
        ((showExtended && extendedAvailable) || !/^f(1[3-9]|2[0-4])$/.test(k.id)),
    ),
  );
  function stop() {
    recording = false;
    clearTimeout(timer);
  }
  onDestroy(stop);
  function commit(next: string[]) {
    const error = validateChord(next);
    message = error ? t(error) : '';
    if (!error) {
      cleared = false;
      onchange(next.map((k) => normalizeKey(k)!));
    }
  }
  function toggle(id: string) {
    const present = selectedKeys.some((k) => normalizeKey(k) === id);
    commit(present ? selectedKeys.filter((k) => normalizeKey(k) !== id) : [...selectedKeys, id]);
  }
</script>

<fieldset>
  <legend>{t('ui_keypicker_shortcut_keys')}</legend>
  <p>
    {t(
      'ui_keypicker_keys_are_pressed_together_on_the_host_desktop_permission_is_required_wayland_asks_thr',
    )}
  </p>
  <p role="status">
    {capabilities
      ? `${capabilities.platform}: ${capabilities.availability}`
      : t('ui_keypicker_host_capabilities_unavailable_extended_keys_are_disabled')}
  </p>
  <div class="chips" aria-label={t('ui_keypicker_current_shortcut')}>
    {#each selectedKeys as key, i}
      <button
        type="button"
        aria-label={t('ui_keypicker_remove', { key: keyLabel(key) })}
        onclick={() => commit(selectedKeys.filter((_, index) => i !== index))}
        >{keyLabel(key)} ×</button
      >
      {#if i < selectedKeys.length - 1}<span>+</span>{/if}
    {/each}
  </div>
  <div class="choices" aria-label={t('ui_keypicker_modifiers')}>
    {#each ['ctrl', 'shift', 'alt', 'meta'] as modifier}<button
        type="button"
        aria-pressed={selectedKeys.some((k) => normalizeKey(k) === modifier)}
        onclick={() => toggle(modifier)}>{keyLabel(modifier)}</button
      >{/each}
  </div>
  <label>{t('ui_keypicker_search_keys')}<input type="search" bind:value={search} /></label>
  <label
    >{t('ui_keypicker_category')}<select bind:value={category}
      >{#each categories as group}<option value={group}>{group}</option>{/each}</select
    ></label
  >
  <label
    ><input type="checkbox" bind:checked={showExtended} disabled={!extendedAvailable} />{t(
      'ui_keypicker_show_f13_f24_requires_host_backend_support',
    )}</label
  >
  <div class="choices">
    {#each visible as key}<button
        type="button"
        disabled={platformSupport(key.id, capabilities?.platform ?? 'windows') === 'unsupported'}
        title={platformSupport(key.id, capabilities?.platform ?? 'windows') === 'unsupported'
          ? t(
              'ui_keypicker_unavailable_in_this_backend_fn_is_normally_handled_by_keyboard_firmware',
            )
          : t(
              'ui_keypicker_implemented_on_windows_x11_and_approved_wayland_portals_desktop_access_is_required',
            )}
        onclick={() =>
          commit([
            ...selectedKeys.filter((k) =>
              ['ctrl', 'shift', 'alt', 'meta'].includes(normalizeKey(k) ?? ''),
            ),
            key.id,
          ])}>{key.label}</button
      >{/each}
  </div>
  <button
    type="button"
    bind:this={capture}
    aria-pressed={recording}
    onclick={() => {
      stop();
      recording = true;
      capture.focus();
      timer = setTimeout(stop, 10000);
    }}
    onblur={stop}
    onkeydown={(event) => {
      if (!recording) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') {
        stop();
        return;
      }
      const chord = captureChord(event);
      if (chord) {
        commit(chord);
        stop();
      }
    }}
    >{recording
      ? t('ui_keypicker_press_shortcut_here_escape_cancels')
      : t('ui_keypicker_record_shortcut')}</button
  >
  {#if recording}<button type="button" onclick={stop}>{t('ui_keypicker_cancel_recording')}</button
    >{/if}
  <button type="button" onclick={() => commit(['ctrl', 'c'])}
    >{t('ui_keypicker_reset_to_ctrl_c')}</button
  >
  <button type="button" onclick={() => commit(['ctrl', 'shift', 'f12'])}
    >{t('ui_keypicker_ctrl_shift_f12')}</button
  >
  <button
    type="button"
    onclick={() => {
      cleared = true;
      message = t('ui_keypicker_choose_between_1_and_16_keys');
    }}>{t('ui_keypicker_clear')}</button
  >
  <details>
    <summary>{t('ui_keypicker_advanced_manual_entry')}</summary>
    <label
      >{t('ui_keypicker_keys_separated_by_commas')}<input
        bind:value={advanced}
        placeholder={t('ui_keypicker_manual_example')}
      /></label
    >
    <button type="button" onclick={() => commit(advanced.split(',').map((k) => k.trim()))}
      >{t('ui_keypicker_use_keys')}</button
    >
  </details>
  <p role="status">{message}</p>
</fieldset>

<style>
  .chips,
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
    margin-block: 0.6rem;
  }
  button {
    min-height: 2.75rem;
  }
  [aria-pressed='true'] {
    outline: 2px solid currentColor;
  }
</style>
