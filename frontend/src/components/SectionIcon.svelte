<script lang="ts">
  import { SECTION_ICONS, type SectionIconName } from './icons';

  /** Distinct 16px stroke glyph per settings section (decorative). */

  interface Props {
    name: SectionIconName;
    size?: number | undefined;
  }

  let { name, size = 16 }: Props = $props();
</script>

<svg
  class="wd2-section-icon"
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <!-- eslint-disable-next-line svelte/no-at-html-tags: static allowlist map, no user input. -->
  {@html SECTION_ICONS[name]}
</svg>
