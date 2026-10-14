<script lang="ts">
  // A searchable, optionally grouped list of rows ({id, label, detail, group}).
  import type { Json } from '../api'
  let {
    rows,
    selected = $bindable(),
    placeholder = 'Search…',
    onselect,
  }: { rows: Json[]; selected?: Json; placeholder?: string; onselect?: (row: Json) => void } = $props()
  let query = $state('')
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase()
    if (!q) return rows
    return rows.filter((r) => `${r.label} ${r.detail ?? ''} ${r.id}`.toLowerCase().includes(q))
  })
  const groups = $derived.by(() => {
    const out: { name: string; rows: Json[] }[] = []
    for (const r of filtered.slice(0, 2000)) {
      const name = r.group ?? ''
      let g = out.find((x) => x.name === name)
      if (!g) out.push((g = { name, rows: [] }))
      g.rows.push(r)
    }
    return out
  })
</script>

<div class="list-pane">
  <div class="search"><input type="search" bind:value={query} {placeholder} aria-label="Search" /></div>
  <div class="list" role="listbox">
    {#each groups as g (g.name)}
      {#if g.name}<div class="group-title">{g.name}</div>{/if}
      {#each g.rows as r (r.id)}
        <button
          class="item"
          class:selected={selected === r.id}
          role="option"
          aria-selected={selected === r.id}
          onclick={() => { selected = r.id; onselect?.(r) }}
        >
          <span class="label">{r.label}</span>
          {#if r.detail}<span class="detail">{r.detail}</span>{/if}
        </button>
      {/each}
    {/each}
    {#if filtered.length === 0}<div class="empty">Nothing matches “{query}”.</div>{/if}
    {#if filtered.length > 2000}<div class="empty">{filtered.length - 2000} more; search to narrow down.</div>{/if}
  </div>
</div>
