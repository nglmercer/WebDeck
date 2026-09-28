<script lang="ts">
  import { CLOSE_PATH } from './icons';

  /**
   * Modal close glyph (bi-x-circle-fill, 19px). Upstream markup carries a
   * duplicated `class` attribute (parser keeps the first); the parsed DOM
   * is a single class, which is what this renders.
   */

  interface Props {
    cls: string;
    dark: string;
  }

  let { cls, dark }: Props = $props();
</script>

<svg
  class="{cls} {dark}"
  xmlns="http://www.w3.org/2000/svg"
  width="19"
  height="19"
  fill="currentColor"
  viewBox="0 0 16 16"><path d={CLOSE_PATH} /></svg>
