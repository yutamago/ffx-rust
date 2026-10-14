<script lang="ts">
  // Event and AI scripts as assembly (advanced).
  import { run, toast } from '../state.svelte'
  import type { Json } from '../api'
  import SearchList from './SearchList.svelte'
  import Icon from './Icon.svelte'

  let rows = $state<Json[]>([])
  let selected = $state<Json>(undefined)
  let script = $state<Json>(null)
  let source = $state('')
  run('script_list').then((r) => (rows = r.rows))
  async function open(id: Json) {
    if (script && source !== script.source && !confirm('Discard the changes you have not saved?')) return
    script = await run('script_get', { path: id })
    source = script.source
  }
  async function save() {
    const r = await run('script_save', { path: script.path, source })
    toast(`Assembled (${r.size} bytes) into your mod`)
    script.source = source
  }
</script>

<div class="split">
  <SearchList {rows} bind:selected onselect={(r) => open(r.id)} placeholder="Search scripts…" />
  <div class="detail-pane" style="display: flex; flex-direction: column">
    {#if script}
      <div class="toolbar">
        <h2 class="grow mono" style="margin: 0; font-size: 14px">{script.path}</h2>
        <button class="primary" disabled={source === script.source} onclick={save}><Icon name="save" />Assemble and save</button>
      </div>
      <p class="muted" style="font-size: 12px">One instruction per line; <span class="mono">eN:</span> and <span class="mono">lN:</span> place entry points and labels; comments after <span class="mono">;</span> name native functions. See docs/formats/script.md.</p>
      <textarea class="mono grow" spellcheck="false" bind:value={source} style="min-height: 400px; white-space: pre"></textarea>
    {:else}
      <div class="empty"><Icon name="code" size={40} /><p>Pick a script to read or change it.</p></div>
    {/if}
  </div>
</div>
