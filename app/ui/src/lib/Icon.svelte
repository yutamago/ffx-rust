<script lang="ts">
  // A small set of line icons (24 × 24 paths).
  let { name, size = 18 }: { name: string; size?: number } = $props()
  const paths: Record<string, string> = {
    home: 'M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10',
    play: 'M7 4l13 8-13 8z',
    potion: 'M9 3h6M10 3v5L5 17a3 3 0 0 0 3 4h8a3 3 0 0 0 3-4l-5-9V3M7 14h10',
    spark: 'M12 2l2.5 7.5L22 12l-7.5 2.5L12 22l-2.5-7.5L2 12l7.5-2.5z',
    gear: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1',
    key: 'M15 7a4 4 0 1 1-3.9 5H3v3h3v3h3v-3h2.1A4 4 0 0 1 15 7z',
    sword: 'M14.5 4.5L19 3l-1.5 4.5L8 17l-3 1 1-3zM5 19l-2 2M7 15l2 2',
    shop: 'M4 7h16l-1 4H5zM5 11v9h14v-9M9 20v-5h6v5M3 7l2-4h14l2 4',
    chest: 'M3 10h18v10H3zM3 10a9 5 0 0 1 18 0M11 13h2v3h-2z',
    monster: 'M5 20c0-6 3-9 7-9s7 3 7 9M8 7l-2-4M16 7l2-4M9 15h.01M15 15h.01M7 8a5 5 0 0 1 10 0',
    swords: 'M4 4l9 9M20 4l-9 9M7 17l-3 3M17 17l3 3M6 15l3 3M18 15l-3 3',
    person: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0',
    chat: 'M4 5h16v11H8l-4 4z',
    code: 'M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16',
    cube: 'M12 2l9 5v10l-9 5-9-5V7zM3 7l9 5 9-5M12 12v10',
    folder: 'M3 6h6l2 2h10v11H3z',
    mods: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
    plus: 'M12 5v14M5 12h14',
    save: 'M5 4h11l3 3v13H5zM8 4v5h7V4M8 20v-6h8v6',
    undo: 'M4 9h11a5 5 0 0 1 0 10H8M4 9l4-4M4 9l4 4',
    export: 'M12 3v12M7 8l5-5 5 5M4 15v5h16v-5',
    import: 'M12 15V3M7 10l5 5 5-5M4 15v5h16v-5',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4',
    check: 'M5 12l5 5 9-10',
    warn: 'M12 3l10 18H2zM12 10v5M12 18h.01',
    settings: 'M4 6h16M4 12h16M4 18h16M8 4v4M16 10v4M10 16v4',
    trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13',
    open: 'M14 4h6v6M20 4l-9 9M18 14v6H4V6h6',
  }
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d={paths[name] ?? paths.cube} />
</svg>
