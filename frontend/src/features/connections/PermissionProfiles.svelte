<script lang="ts">
  import SwitchField from '../../components/SwitchField.svelte';
  import SelectField from '../../components/SelectField.svelte';
  import ChoiceField from '../settings/ui/ChoiceField.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { capabilityNames } from '../../lib/capability-labels';
  import type { Capability } from '../../lib/contracts';
  const t = useTranslations();
  let {
    selected = $bindable<Capability[]>(['read']),
    recommended = ['read'] as Capability[],
    full = [
      'read',
      'input',
      'audio',
      'window',
      'power',
      'script',
      'network',
      'plugin',
    ] as Capability[],
    profile = $bindable('custom'),
    compact = false,
  } = $props();
  const Picker = $derived(compact ? SelectField : ChoiceField);
  const options = $derived([
    { value: 'view', label: t('devices.profileView'), icon: 'eye' },
    { value: 'deck', label: t('devices.profileDeck'), icon: 'grid' },
    { value: 'full', label: t('devices.profileFull'), icon: 'shield' },
    { value: 'custom', label: t('devices.profileCustom'), icon: 'settings' },
  ]);
  function choose(value: string) {
    profile = value;
    if (value === 'view') selected = ['read'];
    if (value === 'deck') selected = [...recommended];
    if (value === 'full') selected = [...full];
  }
</script>

<Picker label={t('devices.permissionProfile')} value={profile} {options} onchange={choose} />
{#if profile === 'full'}<p>{t('devices.fullControllerHelp')}</p>{/if}
{#if profile === 'custom'}<fieldset>
    <legend>{t('devices.permissions')}</legend>
    {#each full as capability}<SwitchField
        label={t.message(capabilityNames[capability] ?? capability)}
        checked={selected.includes(capability)}
        onchange={(checked) => {
          selected = checked
            ? [...selected, capability]
            : selected.filter((value) => value !== capability);
        }}
      />{/each}
  </fieldset>{:else}<div class="capability-tags">
    {#each selected as capability}<span>{t.message(capabilityNames[capability] ?? capability)}</span
      >{/each}
  </div>{/if}

<style>
  .capability-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .capability-tags span {
    font-size: 0.75rem;
    padding: 4px 8px;
    background: var(--surface-hover);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
  }
</style>
