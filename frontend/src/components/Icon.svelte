<script lang="ts">
  let { name, size = 24 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    spotify: 'M5 9c5-2 10-2 14 1M6 13c4-2 8-2 12 1M7 17c3-1 6-1 9 1',
    play: 'M8 5l12 7-12 7z',
    settings:
      'M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1zM12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8',
    folder: 'M3 7V5h6l2 2h10v13H3z',
    fullscreen: 'M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5',
    refresh: 'M20 7v5h-5M20 12a8 8 0 1 0-2 6',
    cpu: 'M7 7h10v10H7zM9 2v5m6-5v5M9 17v5m6-5v5M2 9h5m-5 6h5m10-6h5m-5 6h5',
    memory: 'M3 7h18v10H3zM6 10v4m4-4v4m4-4v4m4-4v4M6 17v3m4-3v3m4-3v3m4-3v3',
    grid: 'M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z',
    monitor: 'M3 3h18v14H3zM8 21h8m-4-4v4',
    plus: 'M12 4v16M4 12h16',
    edit: 'M15 4l5 5M4 16L16 4l4 4L8 20H4z',
    trash: 'M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7',
    close: 'M6 6l12 12M6 18L18 6',
    back: 'M20 12H4m7-7-7 7 7 7',
    home: 'M3 10l9-7 9 7M5 9v12h5v-7h4v7h5V9',
    image: 'M3 3h18v18H3zM3 16l6-6 5 5 3-3 4 4M15 7h.01',
    music: 'M9 18V5l11-2v13M9 5l11-2M9 18a3 3 0 1 1-3-3h3m11 1a3 3 0 1 1-3-3h3',
  };
  const aliases: Record<string, string> = {
    '▶': 'play',
    '▶️': 'play',
    '⚙': 'settings',
    '⚙️': 'settings',
    '📁': 'folder',
    '⛶': 'fullscreen',
    '▦': 'grid',
    '◉': 'cpu',
    '▤': 'memory',
    '▧': 'image',
    '▨': 'cpu',
    '↻': 'refresh',
    '↩': 'back',
    '✦': 'grid',
    '□': 'fullscreen',
    '🎵': 'music',
  };
  const key = $derived(aliases[name] ?? name.replace(/^icon:/, ''));
</script>

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
  >{#if key === 'spotify'}<circle cx="12" cy="12" r="11" fill="#1ed760" stroke="none" />{/if}<path
    d={paths[key] ?? paths.grid}
    stroke={key === 'spotify' ? '#14331f' : key === 'folder' ? '#e7a500' : undefined}
    fill={key === 'folder' ? '#ffbf00' : 'none'}
  /></svg
>
