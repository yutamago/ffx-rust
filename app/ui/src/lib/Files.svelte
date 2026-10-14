<script lang="ts">
  // The game's archive (advanced): browse, extract, add files to the mod.
  import { run, toast } from '../state.svelte'
  import { fileData, type Json } from '../api'
  import Icon from './Icon.svelte'

  let listing = $state<Json>(null)
  let addPath = $state('')
  async function go(prefix: string) {
    listing = await run('file_list', { prefix })
  }
  go('')
  const crumbs = $derived.by(() => {
    const parts = (listing?.prefix ?? '').split('/').filter(Boolean)
    return parts.map((p: string, i: number) => ({ name: p, prefix: parts.slice(0, i + 1).join('/') + '/' }))
  })
  async function extract(path: string) {
    const r = await run('file_extract', { path })
    toast(`Extracted to ${r.file}`)
  }
  async function add(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    if (!file || !addPath) return
    await run('file_add', { path: addPath, name: file.name, data: await fileData(file) })
    toast(`Added ${addPath} to your mod`)
    input.value = ''
  }
</script>

<div class="detail-pane" style="height: 100%">
  <h1>Game files</h1>
  <div class="row" style="margin-bottom: 12px; flex-wrap: wrap">
    <button class="ghost" onclick={() => go('')}><Icon name="home" /></button>
    {#each crumbs as c}<span class="muted">/</span><button class="ghost" onclick={() => go(c.prefix)}>{c.name}</button>{/each}
  </div>
  {#if listing}
    <div class="card" style="padding: 0">
      {#each listing.dirs as d (d.path)}
        <button class="item" style="display: flex; width: 100%; border: none; border-radius: 0; border-bottom: 1px solid var(--line); background: transparent" onclick={() => go(d.path)}>
          <Icon name="folder" /><span class="grow" style="text-align: left">{d.name}</span><span class="muted">{d.files} files</span>
        </button>
      {/each}
      {#each listing.files as f (f.path)}
        <div class="row" style="padding: 6px 14px; border-bottom: 1px solid var(--line)">
          <span class="grow mono">{f.name}</span>
          <span class="muted">{(f.size / 1024).toFixed(1)} KB</span>
          <button onclick={() => extract(f.path)}><Icon name="export" />Extract</button>
          <button onclick={() => (addPath = f.path)}>Replace…</button>
        </div>
      {/each}
    </div>
  {/if}
  <div class="card" style="margin-top: 16px">
    <h2>Add a file to your mod</h2>
    <p class="muted">The file replaces (or adds) the archive path below when the game runs with mods.</p>
    <div class="row">
      <input class="mono" placeholder="archive path, e.g. ffx_ps2/ffx/master/jppc/battle/kernel/takara.bin" bind:value={addPath} />
      <label><span class="chip" style="padding: 7px 14px; cursor: pointer"><Icon name="import" />Choose file…</span><input type="file" style="display: none" onchange={add} disabled={!addPath} /></label>
    </div>
  </div>
</div>
