<script lang="ts">
  // Battle fields → encounter groups → formations and their monsters.
  import { run, toast } from '../state.svelte'
  import type { Json } from '../api'
  import SearchList from './SearchList.svelte'
  import Icon from './Icon.svelte'

  let rows = $state<Json[]>([])
  let onlyRandom = $state(false)
  let selected = $state<Json>(undefined)
  let field = $state<Json>(null)
  // Edits per group: rate and weights.
  let groupEdits = $state<Record<number, { rate: number; weights: number[] }>>({})
  let editing = $state<{ file: string; monsters: (number | null)[]; underwater: boolean } | null>(null)

  async function loadRows() {
    rows = (await run('encounter_fields')).rows
  }
  loadRows()
  const shown = $derived(onlyRandom ? rows.filter((r) => r.random) : rows)

  function load(f: Json) {
    field = f
    groupEdits = {}
    for (const g of f.groups) groupEdits[g.index] = { rate: g.rate, weights: g.formations.map((x: Json) => x.weight) }
  }
  async function open(id: Json) {
    load(await run('encounter_field', { field: id }))
    editing = null
  }
  function changed(g: Json) {
    const e = groupEdits[g.index]
    return e && (e.rate !== g.rate || e.weights.some((w, i) => w !== g.formations[i].weight))
  }
  function chance(g: Json, i: number) {
    const w = groupEdits[g.index].weights
    const total = w.reduce((a, b) => a + b, 0)
    return total ? Math.round((w[i] * 100) / total) : 0
  }
  async function saveGroup(g: Json) {
    const e = groupEdits[g.index]
    load(await run('encounter_save_group', { field: field.id, group: g.index, rate: e.rate, weights: e.weights }))
    toast('Encounter group saved')
    loadRows()
  }
  function editFormation(x: Json) {
    const monsters = Array.from({ length: 8 }, (_, i) => x.monsters[i]?.id ?? null)
    editing = { file: x.file, monsters, underwater: x.flags?.underwater ?? false }
  }
  async function saveFormation() {
    if (!editing) return
    const used = editing.monsters.filter((m) => m !== null)
    load(await run('formation_save', { field: field.id, file: editing.file, monsters: used, underwater: editing.underwater }))
    editing = null
    toast('Formation saved')
  }
</script>

<div class="split">
  <div style="display: flex; flex-direction: column; min-height: 0">
    <label class="row" style="padding: 10px 14px; border-right: 1px solid var(--line); border-bottom: 1px solid var(--line)">
      <input type="checkbox" bind:checked={onlyRandom} /> Only places with random encounters
    </label>
    <SearchList rows={shown} bind:selected onselect={(r) => open(r.id)} placeholder="Search places…" />
  </div>
  <div class="detail-pane">
    {#if field}
      <h1>{field.name} <span class="muted" style="font-size: 14px">map {field.map}</span></h1>
      {#if field.shared_with.length}
        <p class="muted">Shares its encounters with {field.shared_with.join(', ')}: changes apply there too.</p>
      {/if}
      {#each field.groups as g (g.index)}
        <div class="card" style="margin-bottom: 14px">
          <div class="row" style="margin-bottom: 10px">
            <h2 class="grow" style="margin: 0">Group {g.index + 1}</h2>
            <label for={`rate-${g.index}`}>Encounter rate</label>
            <input id={`rate-${g.index}`} type="number" min="0" max="255" style="width: 80px" bind:value={groupEdits[g.index].rate} />
            <span class="muted" style="font-size: 12px">{groupEdits[g.index].rate === 0 ? 'no random battles' : 'higher = more often'}</span>
            <button class="primary" disabled={!changed(g)} onclick={() => saveGroup(g)}><Icon name="save" />Save</button>
          </div>
          <table style="width: 100%; border-collapse: collapse">
            <thead><tr class="muted" style="text-align: left; font-size: 12px"><th>Battle</th><th>Monsters</th><th style="width: 220px">How often</th><th></th></tr></thead>
            <tbody>
              {#each g.formations as x, i (x.position)}
                <tr style="border-top: 1px solid var(--line)">
                  <td class="mono" style="padding: 8px 0">{x.file}</td>
                  <td>
                    {#each x.monsters.filter(Boolean) as m}<span class="chip" style="margin: 2px">{m.name}</span>{/each}
                    {#if x.flags?.underwater}<span class="chip">underwater</span>{/if}
                  </td>
                  <td>
                    <div class="row">
                      <input type="range" min="0" max="15" bind:value={groupEdits[g.index].weights[i]} aria-label="weight" />
                      <span style="width: 70px; text-align: right">{groupEdits[g.index].weights[i] ? `${chance(g, i)} %` : 'never'}</span>
                    </div>
                  </td>
                  <td style="text-align: right"><button onclick={() => editFormation(x)}>Monsters…</button></td>
                </tr>
                {#if editing && editing.file === x.file}
                  <tr><td colspan="4">
                    <div class="card" style="margin: 8px 0; background: var(--panel-2)">
                      <p class="muted" style="margin-top: 0">Up to eight monsters. Empty slots are skipped.</p>
                      <div class="fields">
                        {#each editing.monsters as _, k}
                          <div class="field">
                            <label for={`slot-${k}`}>Slot {k + 1}</label>
                            <select id={`slot-${k}`} bind:value={editing.monsters[k]}>
                              <option value={null}>(empty)</option>
                              {#each field.monsters as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
                            </select>
                          </div>
                        {/each}
                      </div>
                      <div class="row" style="margin-top: 12px">
                        <label class="row grow"><input type="checkbox" bind:checked={editing.underwater} /> Underwater battle</label>
                        <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
                        <button class="primary" onclick={saveFormation}><Icon name="save" />Save formation</button>
                      </div>
                    </div>
                  </td></tr>
                {/if}
              {/each}
            </tbody>
          </table>
        </div>
      {/each}
    {:else}
      <div class="empty"><Icon name="swords" size={40} /><p>Pick a place to see which battles it can start, how often, and with which monsters.</p></div>
    {/if}
  </div>
</div>
