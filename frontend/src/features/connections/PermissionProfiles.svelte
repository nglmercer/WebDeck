<script lang="ts">
  import SwitchField from '../../components/SwitchField.svelte';
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
  } = $props();
  function choose(value: string) {
    profile = value;
    if (value === 'view') selected = ['read'];
    if (value === 'deck') selected = [...recommended];
    if (value === 'full') selected = [...full];
  }
</script>

<ChoiceField
  label={t('devices.permissionProfile')}
  value={profile}
  options={[
    { value: 'view', label: t('devices.profileView') },
    { value: 'deck', label: t('devices.profileDeck') },
    { value: 'full', label: t('devices.profileFull') },
    { value: 'custom', label: t('devices.profileCustom') },
  ]}
  onchange={choose}
/>
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
  </fieldset>{:else}<p>{selected.map((c) => t.message(capabilityNames[c] ?? c)).join(', ')}</p>{/if}
