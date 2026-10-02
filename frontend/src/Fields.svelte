<script lang="ts">
  import Fields from './Fields.svelte';
  import { resolve, defaultValue, type Schema, upload, select } from './api';
  let {
    schema,
    value,
    onchange,
    label = 'Arguments',
  }: { schema: Schema; value: unknown; onchange: (v: unknown) => void; label?: string } = $props();
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
        error = String(e);
      }
    }
  }
  async function picker() {
    try {
      const r = await select('file');
      if (r.source) onchange(r.source);
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if s.oneOf}
  <fieldset>
    <legend>{label}</legend><label
      >Type <select
        value={String(object.type ?? '')}
        onchange={(e) => {
          const option = s.oneOf?.find((o) => o.properties?.type?.const === e.currentTarget.value);
          if (option) onchange(defaultValue(option));
        }}
        >{#each s.oneOf as option}<option value={String(option.properties?.type?.const ?? '')}
            >{String(option.properties?.type?.const ?? '')}</option
          >{/each}</select
      ></label
    >
    {#each s.oneOf.filter((o) => o.properties?.type?.const === object.type) as option}<Fields
        schema={option}
        {value}
        {onchange}
        {label}
      />{/each}
    {#if schema.$ref?.endsWith('/FileSource')}<label
        >Upload <input type="file" onchange={file} /></label
      ><button type="button" onclick={picker}>Select a local file</button>{/if}
  </fieldset>
{:else if s.properties}
  <div class="fields">
    {#each Object.entries(s.properties).filter(([k]) => k !== 'type') as [k, child]}<Fields
        schema={child}
        value={object[k]}
        onchange={(v) => field(k, v)}
        label={k.replaceAll('_', ' ')}
      />{/each}
  </div>
{:else if s.enum}
  <label
    >{label}<select value={String(value)} onchange={(e) => onchange(e.currentTarget.value)}
      >{#each s.enum as v}<option value={String(v)}>{String(v).replaceAll('_', ' ')}</option
        >{/each}</select
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
      min={s.minimum}
      max={s.maximum}
      value={Number(value)}
      onchange={(e) => onchange(e.currentTarget.valueAsNumber)}
    /></label
  >
{:else if s.type === 'array'}
  <fieldset>
    <legend>{label}</legend>{#each Array.isArray(value) ? value : [] as v, index}<div class="row">
        <Fields
          schema={s.items ?? {}}
          value={v}
          onchange={(v) => {
            const next = [...(value as unknown[])];
            next[index] = v;
            onchange(next);
          }}
          label={`${label} ${index + 1}`}
        /><button
          type="button"
          onclick={() => onchange((value as unknown[]).filter((_, i) => i !== index))}
          >Remove</button
        >
      </div>{/each}<button
      type="button"
      onclick={() =>
        onchange([...(Array.isArray(value) ? value : []), defaultValue(s.items ?? {})])}
      >Add {label}</button
    >
  </fieldset>
{:else if s.type === 'object'}
  <fieldset>
    <legend>{label}</legend>{#each Object.entries(object) as [k, v]}<div class="row">
        <input
          aria-label="Name"
          value={k}
          onchange={(e) => {
            const next = { ...object };
            delete next[k];
            next[e.currentTarget.value] = v;
            onchange(next);
          }}
        /><Fields
          schema={typeof s.additionalProperties === 'object'
            ? s.additionalProperties
            : { type: 'string' }}
          value={typeof v === 'string' ? v : JSON.stringify(v)}
          onchange={(v) => field(k, v)}
          label={k}
        /><button
          type="button"
          onclick={() => {
            const next = { ...object };
            delete next[k];
            onchange(next);
          }}>Remove</button
        >
      </div>{/each}<button
      type="button"
      onclick={() => field(`field${Object.keys(object).length + 1}`, '')}>Add field</button
    >
  </fieldset>
{:else}
  <label
    >{label}<textarea
      rows={label.includes('code') || label.includes('body') ? 4 : 1}
      value={typeof value === 'string' ? value : JSON.stringify(value)}
      onchange={(e) => onchange(e.currentTarget.value)}></textarea></label
  >
{/if}
{#if error}<p role="alert">{error}</p>{/if}
