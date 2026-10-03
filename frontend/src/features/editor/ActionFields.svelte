<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Command } from '../../lib/contracts';
  import { commandSchema, defaultValue, type Schema } from '../../lib/schema';
  import Fields from './Fields.svelte';
  let { value, onchange }: { value: Command; onchange: (command: Command) => void } = $props();
  let search = $state('');
  let category = $state('all');
  const categories: Record<string, string> = {
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
  const group = (option: Schema) => option['x-capability'] ?? 'read';
  const names: Record<string, string> = {
    key: 'ui_keyboard_shortcut',
    write: 'ui_type_text',
    open: 'ui_open_app_or_url',
    play_pause: 'ui_play_pause',
    app_volume: 'ui_application_volume',
    fetch: 'ui_http_request',
    plugin: 'ui_plugin_action',
    button: 'ui_run_another_button',
    debug: 'ui_debug_data',
  };
  const descriptions: Record<string, string> = {
    key: 'ui_enter_a_chord_such_as_ctrl_c_desktop_permission_may_be_required',
    write: 'ui_type_text_on_the_host_computer_enable_send_to_press_enter_afterward',
    open: 'ui_open_a_url_application_or_file_on_the_host_computer',
    fetch: 'ui_send_an_http_request_from_the_host_network_permission_is_required',
    script: 'ui_run_a_rhai_script_with_the_permissions_granted_to_the_caller',
    button: 'ui_reference_another_configured_button_by_its_stable_id',
  };
  const options = commandSchema.oneOf ?? [];
  const name = (schema: Schema) => String(schema.properties?.type?.const ?? '');
  const friendly = (type: string) =>
    t(names[type] ?? type.replaceAll('_', ' ').replace(/^./, (c) => c.toUpperCase()));
  const visible = $derived(
    options.filter(
      (option) =>
        name(option) === value.type ||
        ((category === 'all' || group(option) === category) &&
          `${friendly(name(option))} ${name(option)} ${t(descriptions[name(option)] ?? '')}`
            .toLowerCase()
            .includes(search.toLowerCase())),
    ),
  );
  const selected = $derived(options.find((option) => name(option) === value.type));
  const form = $derived(
    value.type === 'plugin' && selected
      ? { ...selected, properties: { type: selected.properties!.type! } }
      : selected,
  );
</script>

<fieldset>
  <legend>{t('ui_command')}</legend>
  <label
    >{t('ui_find_an_action')}<input
      type="search"
      bind:value={search}
      placeholder={t('ui_search_actions')}
    /></label
  >
  <label
    >{t('ui_category')}<select bind:value={category}
      ><option value="all">{t('ui_all_actions')}</option
      >{#each Object.entries(categories) as [id, label]}<option value={id}>{t(label)}</option
        >{/each}</select
    ></label
  >
  <label
    >{t('ui_type')}<select
      value={value.type}
      onchange={(e) => {
        const option = options.find((option) => name(option) === e.currentTarget.value);
        if (option) onchange(defaultValue(option) as Command);
      }}
    >
      {#each Object.entries(categories) as [id, label]}{#if visible.some((option) => group(option) === id)}<optgroup
            label={t(label)}
            >{#each visible.filter((option) => group(option) === id) as option}<option
                value={name(option)}>{friendly(name(option))}</option
              >{/each}</optgroup
          >{/if}{/each}
    </select></label
  >
  {#if descriptions[value.type]}<p class="field-help">{t(descriptions[value.type] ?? '')}</p>{/if}
  {#if value.type === 'key'}
    <fieldset>
      <legend>{t('ui_shortcut_keys')}</legend>
      <p class="field-help">
        {t('ui_keys_are_pressed_together_on_the_host_use_ctrl_shift_alt_or_meta_for_modifiers_e')}
      </p>
      <p id="shortcut-key-limits" class="field-help">{t('ui_shortcut_key_limits')}</p>
      {#each value.keys as key, index}<div class="row">
          <label
            >{t('ui_key_number', { index: index + 1 })}<input
              value={key}
              required
              minlength="1"
              maxlength="32"
              oninput={(event) => {
                if (value.type !== 'key' || !event.currentTarget.checkValidity()) return;
                const keys = [...value.keys];
                keys[index] = event.currentTarget.value;
                onchange({ type: 'key', keys });
              }}
            /></label
          ><button
            type="button"
            disabled={value.keys.length <= 1}
            aria-describedby="shortcut-key-limits"
            onclick={() => {
              if (value.type === 'key')
                onchange({ type: 'key', keys: value.keys.filter((_, i) => i !== index) });
            }}>{t('ui_remove_key_number', { index: index + 1 })}</button
          >
        </div>{/each}
      <button
        type="button"
        disabled={value.keys.length >= 16}
        aria-describedby="shortcut-key-limits"
        onclick={() => {
          if (value.type === 'key') onchange({ type: 'key', keys: [...value.keys, 'enter'] });
        }}>{t('ui_add_key')}</button
      >
      <button type="button" onclick={() => onchange({ type: 'key', keys: ['ctrl', 'c'] })}
        >{t('ui_use_copy_shortcut')}</button
      >
    </fieldset>
  {:else if value.type === 'write'}
    <label
      >{t('ui_text_to_type')}<textarea
        value={value.text}
        maxlength="65536"
        oninput={(event) => {
          if (value.type === 'write') onchange({ ...value, text: event.currentTarget.value });
        }}></textarea></label
    >
    <label class="check"
      ><input
        type="checkbox"
        checked={value.send}
        onchange={(event) => {
          if (value.type === 'write') onchange({ ...value, send: event.currentTarget.checked });
        }}
      />{t('ui_press_enter_after_typing')}</label
    >
  {:else if value.type === 'open'}
    <label
      >{t('ui_application_file_or_url')}<input
        value={value.target}
        required
        minlength="1"
        maxlength="4096"
        placeholder={t('ui_https_example_com')}
        oninput={(event) => {
          if (value.type === 'open' && event.currentTarget.checkValidity())
            onchange({ ...value, target: event.currentTarget.value });
        }}
      /></label
    >
  {:else if form}<Fields
      schema={form}
      {value}
      onchange={(value) => onchange(value as Command)}
      label={t('ui_arguments')}
    />{/if}
</fieldset>
