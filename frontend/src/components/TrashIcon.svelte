<script lang="ts">
  import { TRASH_PATHS } from './icons';

  /** Background-picker trash glyph (bi-trash, 16px) with a `<title>`. */

  interface Props {
    title: string;
    onclick?: ((event: MouseEvent) => void) | undefined;
  }

  let { title, onclick }: Props = $props();
</script>

<svg
  {onclick}
  role="button"
  tabindex="0"
  aria-label={title}
  onkeydown={(event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.currentTarget.dispatchEvent(new MouseEvent('click', { bubbles: true })); } }}
  class="choose-bg-delete-button"
  xmlns="http://www.w3.org/2000/svg"
  width="16"
  height="16"
  fill="currentColor"
  viewBox="0 0 16 16"><title>{' '}{title}{' '}</title><path d={TRASH_PATHS[0]} /><path d={TRASH_PATHS[1]} /></svg>
