<script lang="ts">
  import KeySelector from '../src/features/editor/KeySelector.svelte';

  let {
    onchange = () => {},
    onStep = () => {},
  }: {
    onchange?: (keys: string[]) => void;
    onStep?: (open: boolean) => void;
  } = $props();
  let value = $state(['Control', 'c']);
  let active = $state(true);

  function update(keys: string[]) {
    onchange(keys);
    value = keys;
  }
</script>

<button
  type="button"
  aria-label="External update"
  data-testid="external-update"
  onclick={() => (value = ['alt', 'f24'])}
></button>
<button
  type="button"
  aria-label="Toggle active"
  data-testid="toggle-active"
  onclick={() => (active = !active)}
></button>
<KeySelector {value} onchange={update} {active} onMobileStep={onStep} />
