<script lang="ts">
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
    /** Already-normalized `#rrggbb` or '' (see `normalizeHexColor`). */
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
    value,
  }: Props = $props();

  // Empty class segments (editor hex inputs carry only the theme class)
  // collapse away so no stray whitespace lands in `class`.
  const colorCls = $derived([colorClass, dark].filter((c) => c !== '').join(' '));
  const hexCls = $derived([hexClass, dark].filter((c) => c !== '').join(' '));
  const val = $derived(value !== '' ? value : undefined);
</script>

<div class={containerClass}>
  <input type="color" class={colorCls} id={colorId} value={val} />
  <input type="text" id={hexId} name={hexName} class={hexCls} {placeholder} value={val} />
</div>
