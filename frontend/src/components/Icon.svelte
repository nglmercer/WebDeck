<script lang="ts">
  import { registeredIcon } from '../lib/icons';
  let { name, size = 24 }: { name: string; size?: number } = $props();
  const icon = $derived(registeredIcon(name));
</script>

{#if icon}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    {#if icon.background}<circle cx="12" cy="12" r="11" fill={icon.background} stroke="none" />{/if}
    <path d={icon.path} stroke={icon.stroke} fill={icon.fill ?? 'none'} />
  </svg>
{:else}
  <span aria-hidden="true" style:font-size={`${size}px`}
    >{!name.startsWith('icon:') && [...name].length <= 4 ? name : '◇'}</span
  >
{/if}
