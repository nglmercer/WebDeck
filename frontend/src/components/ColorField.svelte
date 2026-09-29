<script lang="ts">
  import { normalizeHexValue } from './colors';

  /** Synced color-picker + HEX text pair. */
  interface Props {
    dark: string;
    containerClass: string;
    colorClass: string;
    colorId: string;
    hexClass: string;
    hexId: string;
    hexName?: string | undefined;
    placeholder?: string | undefined;
    /**
     * Already-normalized `#rrggbb` or '' (see `normalizeHexColor`).
     * Bindable so composers (background picker) can read the live value;
     * unbound parents simply never see keystrokes propagate out.
     */
    value: string;
  }

  let {
    dark,
    containerClass,
    colorClass,
    colorId,
    hexClass,
    hexId,
    hexName,
    placeholder,
    value = $bindable(),
  }: Props = $props();

  // Empty class segments (editor hex inputs carry only the theme class)
  // collapse away so no stray whitespace lands in `class`.
  const colorCls = $derived([colorClass, dark].filter((c) => c !== '').join(' '));
  const hexCls = $derived([hexClass, dark].filter((c) => c !== '').join(' '));

  // Two-way sync (was legacy initColorSetting + the background-setting
  // color block): typing in either input normalizes and mirrors into
  // both. Render-once seed — parents never change `value` after mount.
  const val = $derived(value !== '' ? value : undefined);

  function sync(event: Event): void {
    value = normalizeHexValue((event.currentTarget as HTMLInputElement).value);
  }
</script>

<div class={containerClass}>
  <input type="color" class={colorCls} id={colorId} value={val} oninput={sync} />
  <input type="text" id={hexId} name={hexName} class={hexCls} {placeholder} value={val} oninput={sync} />
</div>
