<script lang="ts">
  import FileSourceField from './FileSourceField.svelte';
  import ScriptSourceField from './ScriptSourceField.svelte';
  import VolumeField from './VolumeField.svelte';
  import MapField from './MapField.svelte';
  import SwitchField from '../../components/SwitchField.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { FileSource, ScriptSource, VolumeChange } from '../../lib/contracts';
  import { fieldLabels } from '../../lib/field-labels';
  import { useTranslations, common } from '../../lib/i18n';
  const t = useTranslations();
  const fieldLabel = (name: string) => {
    const key = fieldLabels[name];
    return key ? t(key) : name.replaceAll('_', ' ').replace(/^./, (value) => value.toUpperCase());
  };

  import Fields from './Fields.svelte';
  import JsonField from './JsonField.svelte';
  import { id } from '../../lib/id';
  const errorId = `field-${id()}`;
  const minimumId = `${errorId}-minimum`;
  const maximumId = `${errorId}-maximum`;
  import { resolve, defaultValue, type Schema } from '../../lib/api/client';
  let {
    schema,
    value,
    onchange,
    label = t('editor.arguments'),
    fieldName = '',
  }: {
    schema: Schema;
    value: unknown;
    onchange: (v: unknown) => void;
    label?: string;
    fieldName?: string;
  } = $props();
  const s = $derived(resolve(schema));
  const object = $derived(
    value && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : {},
  );
  function field(k: string, v: unknown) {
    onchange({ ...object, [k]: v });
  }
  let error = $state('');
</script>

{#if schema.$ref?.endsWith('/FileSource')}
  <FileSourceField
    {label}
    value={(value as FileSource) ?? { type: 'asset', id: '' }}
    onchange={(source) => onchange(source)}
  />
{:else if schema.$ref?.endsWith('/ScriptSource')}
  <ScriptSourceField
    value={(value as ScriptSource) ?? { type: 'inline', code: '' }}
    onchange={(source) => onchange(source)}
  />
{:else if schema.$ref?.endsWith('/VolumeChange')}
  <VolumeField
    value={(value as VolumeChange) ?? { type: 'set', percent: 0 }}
    onchange={(change) => onchange(change)}
  />
{:else if s.oneOf}
  <fieldset>
    <legend>{label}</legend><label
      >{t('editor.type')}
      <select
        value={String(object.type ?? '')}
        onchange={(e) => {
          const option = s.oneOf?.find((o) => o.properties?.type?.const === e.currentTarget.value);
          if (option) onchange(defaultValue(option));
        }}
        >{#each s.oneOf as option}<option value={String(option.properties?.type?.const ?? '')}
            >{fieldLabel(String(option.properties?.type?.const ?? ''))}</option
          >{/each}</select
      ></label
    >
    {#each s.oneOf.filter((o) => o.properties?.type?.const === object.type) as option}<Fields
        schema={option}
        {value}
        {onchange}
        {label}
        {fieldName}
      />{/each}
  </fieldset>
{:else if s.properties}
  <div class="fields">
    {#each Object.entries(s.properties).filter(([k, child]) => k !== 'type' && !(k === 'language' && child.enum?.length === 1 && child.enum[0] === 'javascript')) as [k, child]}<Fields
        schema={child}
        value={object[k]}
        onchange={(v) => field(k, v)}
        label={fieldLabel(k)}
        fieldName={k}
      />{/each}
  </div>
{:else if s.enum}
  <label
    >{label}<select value={String(value)} onchange={(e) => onchange(e.currentTarget.value)}
      >{#each s.enum as v}<option value={String(v)}>{fieldLabel(String(v))}</option>{/each}</select
    ></label
  >
{:else if s.type === 'boolean'}
  <SwitchField {label} checked={Boolean(value)} onchange={(next) => onchange(next)} />
{:else if s.type === 'integer' || s.type === 'number'}
  <label
    >{label}<input
      type="number"
      required
      step={s.type === 'integer' ? 1 : 'any'}
      min={s.minimum}
      max={s.maximum}
      value={Number(value)}
      aria-invalid={!!error}
      aria-describedby={error ? errorId : undefined}
      onchange={(e) => {
        const input = e.currentTarget;
        input.setCustomValidity('');
        if (!Number.isFinite(input.valueAsNumber) || !input.checkValidity()) {
          error = t(common.invalidNumber, { label });
          input.setCustomValidity(error);
          return;
        }
        error = '';
        onchange(input.valueAsNumber);
      }}
    /></label
  >
{:else if s.type === 'array'}
  <fieldset>
    <legend>{label}</legend>
    {#if s.minItems !== undefined}<p id={minimumId} class="field-help">
        {t('editor.listMinimum', { count: s.minItems })}
      </p>{/if}
    {#if s.maxItems !== undefined}<p id={maximumId} class="field-help">
        {t('editor.listMaximum', { count: s.maxItems })}
      </p>{/if}
    {#each Array.isArray(value) ? value : [] as v, index}<div class="row">
        <Fields
          schema={s.items ?? {}}
          value={v}
          onchange={(v) => {
            const next = [...(value as unknown[])];
            next[index] = v;
            onchange(next);
          }}
          label={t(common.number, { label, index: index + 1 })}
        /><button
          type="button"
          disabled={(value as unknown[]).length <= (s.minItems ?? 0)}
          aria-describedby={s.minItems !== undefined ? minimumId : undefined}
          onclick={() => onchange((value as unknown[]).filter((_, i) => i !== index))}
          ><Icon name="close" size={18} /><span class="sr-only">{t(common.remove)}</span></button
        >
      </div>{/each}<button
      type="button"
      disabled={(Array.isArray(value) ? value.length : 0) >= (s.maxItems ?? Infinity)}
      aria-describedby={s.maxItems !== undefined ? maximumId : undefined}
      onclick={() =>
        onchange([...(Array.isArray(value) ? value : []), defaultValue(s.items ?? {})])}
      ><Icon name="plus" size={18} />{t('editor.addNamedField', { label })}</button
    >
  </fieldset>
{:else if s.type === 'object'}
  <MapField
    {label}
    value={object}
    stringValues={typeof s.additionalProperties === 'object' &&
      s.additionalProperties.type === 'string'}
    onchange={(next) => onchange(next)}
  />
{:else if !s.type}
  <JsonField {value} {onchange} {label} />
{:else}
  <label
    >{label}<textarea
      rows={fieldName === 'code' || fieldName === 'body' ? 4 : 1}
      value={typeof value === 'string' ? value : JSON.stringify(value)}
      minlength={s.minLength}
      maxlength={s.maxLength}
      required={(s.minLength ?? 0) > 0}
      aria-invalid={!!error}
      aria-describedby={error ? errorId : undefined}
      oninput={(e) => {
        e.currentTarget.setCustomValidity('');
        const next = e.currentTarget.value;
        if (!e.currentTarget.checkValidity() || (s.pattern && !new RegExp(s.pattern).test(next))) {
          error = t(common.invalidValue, { label });
          e.currentTarget.setCustomValidity(error);
          return;
        }
        error = '';
        onchange(next);
      }}></textarea></label
  >
{/if}
{#if error}<p id={errorId} role="alert">{t.message(error)}</p>{/if}

<style>
  .fields {
    display: grid;
    gap: 0 12px;
  }
</style>
