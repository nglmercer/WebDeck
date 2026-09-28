<script lang="ts">
  import { TRASH_PATHS } from './icons';

  /** Background-picker trash glyph (bi-trash, 16px) with a `<title>`. */

  interface Props {
    title: string;
  }

  let { title }: Props = $props();
</script>

<svg
  class="choose-bg-delete-button"
  xmlns="http://www.w3.org/2000/svg"
  width="16"
  height="16"
  fill="currentColor"
  viewBox="0 0 16 16"><title>{' '}{title}{' '}</title><path d={TRASH_PATHS[0]} /><path d={TRASH_PATHS[1]} /></svg>
