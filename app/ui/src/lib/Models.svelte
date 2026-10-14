<script lang="ts">
  // Models and textures: export for editing in Blender or a paint program,
  // import the edits back.
  import { run, toast } from '../state.svelte'
  import { fileData, type Json } from '../api'
  import SearchList from './SearchList.svelte'
  import Icon from './Icon.svelte'

  let filter = $state('chr/pc/')
  let rows = $state<Json[]>([])
  let selected = $state<Json>(undefined)
  let row = $state<Json>(null)
  let result = $state<Json>(null)

  async function search() {
    rows = (await run('model_list', { filter })).rows
  }
  search()
  async function exportIt() {
    result = await run(row.texture ? 'texture_export' : 'model_export', { path: row.id })
    toast('Exported')
  }
  async function importIt(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    const data = await fileData(file)
    result = await run(row.texture ? 'texture_import' : 'model_import', { path: row.id, name: file.name, data })
    input.value = ''
    toast('Imported into your mod')
  }
  const exported = $derived(result?.file ?? result?.png)
</script>

<div class="split">
  <div style="display: flex; flex-direction: column; min-height: 0; border-right: 1px solid var(--line)">
    <div class="row" style="padding: 10px 12px; border-bottom: 1px solid var(--line)">
      <select bind:value={filter} onchange={search} aria-label="Kind">
        <option value="chr/pc/">Party</option>
        <option value="chr/mon/">Monsters</option>
        <option value="chr/npc/">People</option>
        <option value="chr/sum/">Aeons</option>
        <option value="chr/wep/">Weapons</option>
        <option value="/map/">Maps</option>
        <option value="/btlmap/">Battle maps</option>
        <option value="/tex/">All textures</option>
      </select>
    </div>
    <SearchList {rows} bind:selected onselect={(r) => { row = r; result = null }} placeholder="Search models and textures…" />
  </div>
  <div class="detail-pane">
    {#if row}
      <h1>{row.label}</h1>
      <p class="muted mono">{row.detail}</p>
      <div class="card" style="max-width: 640px">
        {#if row.texture}
          <h2>Texture</h2>
          <p>Export saves a PNG to look at and a DDS to edit (in GIMP, Paint.NET or Photoshop with a DDS plug-in). Import a DDS of the <b>same size and format</b>.</p>
        {:else}
          <h2>Model</h2>
          <p>Export saves a glTF (.glb) with its textures, for Blender and other 3D tools. Edit the shape, add new parts, change weights; then import the .glb. New parts become new segments; a new material can point to another texture.</p>
        {/if}
        <div class="row" style="margin-top: 12px">
          <button class="primary" onclick={exportIt}><Icon name="export" />Export</button>
          <label class="row" style="cursor: pointer">
            <span class="chip" style="padding: 7px 14px; font-size: 14px; color: var(--text)"><Icon name="import" />Import…</span>
            <input type="file" accept={row.texture ? '.dds' : '.glb'} style="display: none" onchange={importIt} />
          </label>
        </div>
        {#if exported}
          <p style="margin-top: 14px">Exported to <span class="mono">{exported}</span></p>
          <button onclick={() => run('open_folder', { path: exported })}><Icon name="open" />Open folder</button>
        {/if}
        {#if result?.segments !== undefined && !result?.file}
          <p style="margin-top: 14px"><Icon name="check" /> {result.segments} parts written{result.new_segments ? `, ${result.new_segments} new` : ''}.</p>
          {#each result.notes ?? [] as n}<p class="muted">{n}</p>{/each}
        {/if}
      </div>
    {:else}
      <div class="empty"><Icon name="cube" size={40} /><p>Pick a model or texture.</p></div>
    {/if}
  </div>
</div>
