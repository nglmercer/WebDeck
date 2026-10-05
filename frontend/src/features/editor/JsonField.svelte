<script lang="ts">
  import { id } from '../../lib/id';
  const errorId = `json-${id()}`;
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    value,
    onchange,
    label,
  }: { value: unknown; onchange: (value: unknown) => void; label: string } = $props();
  let error = $state('');
</script>

<label
  >{label}<textarea
    rows="3"
    value={JSON.stringify(value, null, 2)}
    onchange={(event) => {
      try {
        const next: unknown = JSON.parse(event.currentTarget.value);
        onchange(next);
        error = '';
        event.currentTarget.setCustomValidity('');
      } catch {
        error = 'Enter valid JSON, for example true, 42, "text", or {"key": "value"}.';
        event.currentTarget.setCustomValidity(t(error));
      }
    }}
    aria-invalid={!!error}
    aria-describedby={error ? errorId : undefined}></textarea></label
>
{#if error}<p id={errorId} role="alert">{t(error)}</p>{/if}
