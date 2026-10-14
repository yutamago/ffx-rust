<script lang="ts">
  // Lines of a scene or battle, in the chosen language.
  import { run, toast, app } from '../state.svelte'
  import type { Json } from '../api'
  import SearchList from './SearchList.svelte'
  import Icon from './Icon.svelte'

  let rows = $state<Json[]>([])
  let selected = $state<Json>(undefined)
  let table = $state<Json>(null)
  let edits = $state<Record<number, string>>({})
  let query = $state('')

  run('dialogue_list').then((r) => (rows = r.rows))
  async function open(id: Json) {
    if (Object.keys(edits).length && !confirm('Discard the changes you have not saved?')) return
    table = await run('dialogue_get', { name: id })
    edits = {}
  }
  async function save() {
    const messages = Object.entries(edits).map(([index, text]) => ({ index: Number(index), text }))
    table = await run('dialogue_save', { name: table.name, messages })
    edits = {}
    toast('Dialogue saved')
  }
  // Text split into plain parts and {codes}, for the preview.
  function parts(text: string) {
    return text.split(/(\{[^}]*\})/).filter(Boolean).map((p) => ({ code: p.startsWith('{'), p }))
  }
  const shown = $derived(
    (table?.messages ?? []).filter((m: Json) => m.text !== null && (!query || (edits[m.index] ?? m.text).toLowerCase().includes(query.toLowerCase()))),
  )
</script>

<div class="split">
  <SearchList {rows} bind:selected onselect={(r) => open(r.id)} placeholder="Search scenes and battles…" />
  <div class="detail-pane">
    {#if table}
      <div class="toolbar">
        <div class="grow">
          <h2 style="margin: 0">{table.name}</h2>
          <span class="muted">{table.messages.length} lines · {app.status.language}</span>
        </div>
        <input type="search" placeholder="Find text…" bind:value={query} style="width: 200px" />
        <button class="primary" disabled={!Object.keys(edits).length} onclick={save}><Icon name="save" />Save</button>
      </div>
      <p class="muted" style="font-size: 12px">
        Codes in braces, like <span class="mono">{'{09:30}'}</span>, control colours, pauses, names and choices: keep them. A line break starts a new line in the box.
      </p>
      {#each shown as m (m.index)}
        <div class="card" style="margin-bottom: 10px; padding: 12px" class:changed={m.index in edits}>
          <div class="row" style="margin-bottom: 6px">
            <span class="chip">#{m.index}</span>
            {#if m.choices}<span class="chip">{m.choices} choices</span>{/if}
            <span class="grow"></span>
            {#if m.index in edits}<button class="ghost" onclick={() => delete edits[m.index]}><Icon name="undo" />Undo</button>{/if}
          </div>
          <textarea
            rows={Math.max(2, (edits[m.index] ?? m.text).split('\n').length)}
            value={edits[m.index] ?? m.text}
            style={m.index in edits ? 'border-color: var(--gold)' : ''}
            oninput={(e) => {
              const v = e.currentTarget.value
              if (v === m.text) delete edits[m.index]
              else edits[m.index] = v
            }}
          ></textarea>
          <div style="margin-top: 6px; white-space: pre-wrap; font-size: 13px">
            {#each parts(edits[m.index] ?? m.text) as p}{#if p.code}<span class="mono" style="color: var(--accent); opacity: 0.6; font-size: 11px">{p.p}</span>{:else}{p.p}{/if}{/each}
          </div>
        </div>
      {/each}
    {:else}
      <div class="empty"><Icon name="chat" size={40} /><p>Pick a scene or battle to read and change what is said.</p></div>
    {/if}
  </div>
</div>
