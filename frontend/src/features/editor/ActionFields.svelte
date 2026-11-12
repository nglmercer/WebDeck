<script lang="ts">
  import HttpFields from './HttpFields.svelte';
  import ScriptSourceField from './ScriptSourceField.svelte';
  import VolumeField from './VolumeField.svelte';
  import ApplicationField from './ApplicationField.svelte';
  import SwitchField from '../../components/SwitchField.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import KeySelector from './KeySelector.svelte';
  import AudioFields from './AudioFields.svelte';
  import type { MediaMetadata } from './audio';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Command } from '../../lib/contracts';
  import { commandSchema, defaultValue, type Schema } from '../../lib/schema';
  import Fields from './Fields.svelte';
  let {
    value,
    onchange,
    configurationOnly = false,
    tutorial,
    active = true,
    onKeyPickerStep,
    mediaMetadata,
    onMediaMetadata,
    onValidity = () => {},
  }: {
    value: Command;
    onchange: (command: Command) => void;
    configurationOnly?: boolean;
    tutorial?:
      { step: string; detail?: string | undefined; audio?: AudioBuffer | undefined } | undefined;
    active?: boolean;
    onKeyPickerStep?: (open: boolean) => void;
    mediaMetadata?: MediaMetadata | undefined;
    onMediaMetadata?: (metadata: MediaMetadata) => void;
    onValidity?: (valid: boolean) => void;
  } = $props();
  $effect(() => {
    if (value.type !== 'play_sound') onValidity(true);
  });
  let search = $state('');
  let category = $state('all');
  const group = (option: Schema) => option['x-capability'] ?? 'read';
  import { categories, names, descriptions } from './action-options';
  const options = commandSchema.oneOf ?? [];
  const name = (schema: Schema) => String(schema.properties?.type?.const ?? '');
  const friendly = (type: string) =>
    t.message(names[type] ?? type.replaceAll('_', ' ').replace(/^./, (c) => c.toUpperCase()));
  const visible = $derived(
    options.filter(
      (option) =>
        name(option) === value.type ||
        ((category === 'all' || group(option) === category) &&
          `${friendly(name(option))} ${name(option)} ${t.message(descriptions[name(option)] ?? '')}`
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

<fieldset class:configuration-only={configurationOnly}>
  {#if !configurationOnly}
    <legend>{t('actions.command')}</legend>
    <label
      >{t('actions.findAnAction')}<input
        type="search"
        bind:value={search}
        placeholder={t('actions.searchActions')}
      /></label
    >
    <label
      >{t('actions.category')}<select bind:value={category}
        ><option value="all">{t('actions.allActions')}</option
        >{#each Object.entries(categories) as [id, label]}<option value={id}>{t(label)}</option
          >{/each}</select
      ></label
    >
    <label
      >{t('editor.type')}<select
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
  {/if}
  {#if !configurationOnly && descriptions[value.type]}<p class="field-help">
      {t.message(descriptions[value.type] ?? '')}
    </p>{/if}
  {#if value.type === 'fetch'}
    <HttpFields {value} onchange={(next) => onchange(next)} />
  {:else if value.type === 'script' || value.type === 'shell'}
    <ScriptSourceField
      value={value.source}
      language={value.type === 'script' ? 'javascript' : 'shell'}
      metadata={mediaMetadata}
      onmetadata={onMediaMetadata}
      onchange={(source) => {
        if (value.type === 'script' || value.type === 'shell') onchange({ ...value, source });
      }}
    />
    {#if value.type === 'shell'}<NumberField
        label={t('forms.http.timeout')}
        value={value.timeout_seconds}
        min={1}
        max={30}
        unit="s"
        onchange={(timeout_seconds) => {
          if (value.type === 'shell') onchange({ ...value, timeout_seconds });
        }}
      />{/if}
  {:else if value.type === 'app_volume'}
    <ApplicationField
      label={t('forms.apps.label')}
      value={value.application}
      mode="audio"
      onchange={(application) => {
        if (value.type === 'app_volume') onchange({ ...value, application });
      }}
    />
    <VolumeField
      value={value.change}
      onchange={(change) => {
        if (value.type === 'app_volume') onchange({ ...value, change });
      }}
    />
  {:else if value.type === 'volume'}
    <VolumeField value={value.change} onchange={(change) => onchange({ type: 'volume', change })} />
  {:else if value.type === 'kill' || value.type === 'restart'}
    <ApplicationField
      label={t('forms.apps.label')}
      value={value.target}
      onchange={(target) => {
        if (value.type === 'kill' || value.type === 'restart') onchange({ ...value, target });
      }}
    />
  {:else if value.type === 'play_sound'}
    <AudioFields
      {value}
      {active}
      previewBuffer={tutorial?.audio}
      onchange={(command) => onchange(command)}
      metadata={mediaMetadata}
      onmetadata={onMediaMetadata}
      onvalidity={onValidity}
    />
  {:else if value.type === 'key'}
    <KeySelector
      guidePicker={tutorial ? tutorial.detail === 'key-picker' : undefined}
      guideStep={tutorial?.step}
      value={value.keys}
      onchange={(keys) => onchange({ type: 'key', keys })}
      {active}
      onMobileStep={onKeyPickerStep}
    />
  {:else if value.type === 'write'}
    <label
      >{t('editor.textToType')}<textarea
        value={value.text}
        maxlength="65536"
        oninput={(event) => {
          if (value.type === 'write') onchange({ ...value, text: event.currentTarget.value });
        }}></textarea></label
    >
    <SwitchField
      label={t('editor.pressEnterAfterTyping')}
      checked={value.send}
      onchange={(send) => {
        if (value.type === 'write') onchange({ ...value, send });
      }}
    />
  {:else if value.type === 'open'}
    <ApplicationField
      label={t('editor.applicationFileOrUrl')}
      value={value.target}
      mode="open"
      onchange={(target) => onchange({ type: 'open', target })}
    />
  {:else if form}<Fields
      schema={form}
      {value}
      onchange={(value) => onchange(value as Command)}
      label={t('editor.arguments')}
    />{/if}
</fieldset>

<style>
  .configuration-only {
    display: grid;
    gap: 16px;
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
</style>
