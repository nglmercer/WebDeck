<script lang="ts">
  import SelectField from '../../components/SelectField.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import CodeEditor from '../../components/CodeEditor.svelte';
  import MapField from './MapField.svelte';
  import { useTranslations } from '../../lib/i18n';
  import type { Command } from '../../lib/contracts';
  const t = useTranslations();
  type Http = Extract<Command, { type: 'fetch' }>;
  let { value, onchange }: { value: Http; onchange: (command: Http) => void } = $props();
  const methods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;
  const jsonBody = $derived(
    Object.entries(value.headers).some(
      ([key, value]) =>
        key.toLowerCase() === 'content-type' && value.toLowerCase().includes('json'),
    ) || /^[\s]*[\[{]/.test(value.body),
  );
</script>

<div class="http-fields">
  <div class="request">
    <SelectField
      label={t('forms.http.method')}
      value={value.method}
      options={methods.map((method) => ({ value: method, label: method, icon: 'globe' }))}
      onchange={(method) => onchange({ ...value, method: method as Http['method'] })}
    /><label
      >{t('forms.http.url')}<input
        type="url"
        required
        minlength="1"
        maxlength="4096"
        pattern="https?://.+"
        value={value.url}
        placeholder={t('editor.httpsExampleCom')}
        oninput={(event) => onchange({ ...value, url: event.currentTarget.value })}
      /></label
    >
  </div>
  <MapField
    label={t('forms.http.headers')}
    value={value.headers}
    stringValues
    headers
    onchange={(headers) => onchange({ ...value, headers: headers as Record<string, string> })}
  />
  <CodeEditor
    label={t('forms.http.body')}
    value={value.body}
    language={jsonBody ? 'json' : 'text'}
    onchange={(body) => onchange({ ...value, body })}
  />
  <div class="timeout">
    <NumberField
      label={t('forms.http.timeout')}
      value={value.timeout_seconds}
      min={1}
      max={30}
      unit="s"
      onchange={(timeout_seconds) => onchange({ ...value, timeout_seconds })}
    />
  </div>
</div>

<style>
  .http-fields {
    display: grid;
    gap: 16px;
    min-width: 0;
  }
  .request {
    display: grid;
    grid-template-columns: 125px minmax(0, 1fr);
    gap: 10px;
    align-items: end;
  }
  .request label {
    margin: 0;
    min-width: 0;
    font-size: 0.8rem;
    line-height: normal;
    color: var(--text-muted);
  }
  .request input {
    display: block;
    width: 100%;
    min-height: 44px;
    margin-top: 6px;
    font-size: 0.82rem;
  }
  .timeout {
    max-width: 190px;
  }
  @media (max-width: 380px) {
    .request {
      grid-template-columns: 105px minmax(0, 1fr);
    }
  }
</style>
