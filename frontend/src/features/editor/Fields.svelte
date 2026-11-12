<script lang="ts">
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
  import { resolve, defaultValue, type Schema, upload, select } from '../../lib/api/client';
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
  async function file(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) {
      try {
        onchange(await upload(f));
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
    }
  }
  async function picker() {
    try {
      const r = await select('file');
      if (r.source) onchange(r.source);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

{#if s.oneOf}
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
    {#if schema.$ref?.endsWith('/FileSource')}<label
        >{t(common.upload)} <input type="file" onchange={file} /></label
      ><button type="button" onclick={picker}>{t('editor.selectALocalFile')}</button>{/if}
  </fieldset>
{:else if s.properties}
  <div class="fields">
    {#each Object.entries(s.properties).filter(([k]) => k !== 'type') as [k, child]}<Fields
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
  <label class="check"
    ><input
      type="checkbox"
      checked={Boolean(value)}
      onchange={(e) => onchange(e.currentTarget.checked)}
    />{label}</label
  >
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
          >{t(common.remove)}</button
        >
      </div>{/each}<button
      type="button"
      disabled={(Array.isArray(value) ? value.length : 0) >= (s.maxItems ?? Infinity)}
      aria-describedby={s.maxItems !== undefined ? maximumId : undefined}
      onclick={() =>
        onchange([...(Array.isArray(value) ? value : []), defaultValue(s.items ?? {})])}
      >{t('editor.addNamedField', { label })}</button
    >
  </fieldset>
{:else if s.type === 'object'}
  <fieldset>
    <legend>{label}</legend>{#each Object.entries(object) as [k, v]}<div class="row">
        <input
          aria-label={t(common.name)}
          value={k}
          onchange={(e) => {
            const next = { ...object };
            delete next[k];
            next[e.currentTarget.value] = v;
            onchange(next);
          }}
        /><Fields
          schema={typeof s.additionalProperties === 'object' ? s.additionalProperties : {}}
          value={v}
          onchange={(v) => field(k, v)}
          label={k}
        /><button
          type="button"
          onclick={() => {
            const next = { ...object };
            delete next[k];
            onchange(next);
          }}>{t(common.remove)}</button
        >
      </div>{/each}<button
      type="button"
      onclick={() => field(`field${Object.keys(object).length + 1}`, '')}
      >{t('editor.addField')}</button
    >
  </fieldset>
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
