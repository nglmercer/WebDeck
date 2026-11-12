<script lang="ts">
  import SegmentedControl from '../../components/SegmentedControl.svelte';
  import RangeField from '../../components/RangeField.svelte';
  import { useTranslations } from '../../lib/i18n';
  import type { VolumeChange } from '../../lib/contracts';
  const t = useTranslations();
  let { value, onchange }: { value: VolumeChange; onchange: (change: VolumeChange) => void } =
    $props();
</script>

<div class="volume-editor">
  <SegmentedControl
    label={t('forms.volume.mode')}
    value={value.type}
    options={[
      { value: 'set', label: t('fields.set') },
      { value: 'adjust', label: t('fields.adjust') },
    ]}
    onchange={(type) =>
      onchange(
        type === 'set'
          ? { type: 'set', percent: Math.max(0, value.percent) }
          : { type: 'adjust', percent: value.percent },
      )}
  /><RangeField
    label={t('audio.volume')}
    value={value.percent}
    min={value.type === 'adjust' ? -100 : 0}
    max={100}
    unit="%"
    onchange={(percent) =>
      onchange(value.type === 'set' ? { type: 'set', percent } : { type: 'adjust', percent })}
  />
</div>

<style>
  .volume-editor {
    display: grid;
    gap: 16px;
    min-width: 0;
  }
</style>
