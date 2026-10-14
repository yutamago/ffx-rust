<script lang="ts">
  // List and form for one kind of record: kernel tables or monsters.
  import { run, toast, app } from '../state.svelte'
  import type { Json } from '../api'
  import SearchList from './SearchList.svelte'
  import FieldForm from './FieldForm.svelte'
  import Icon from './Icon.svelte'
  let { editor }: { editor: Json } = $props()

  const isMonsters = $derived(editor.kind === 'monsters')
  let rows = $state<Json[]>([])
  let selected = $state<Json>(undefined)
  let record = $state<Json>(null)
  let changes = $state<Record<string, Json>>({})

  const args = (extra: Json = {}) => (isMonsters ? extra : { table: editor.table, ...extra })
  async function loadRows() {
    const r = await run(isMonsters ? 'monster_rows' : 'table_rows', args())
    rows = r.rows
  }
  async function open(id: Json) {
    if (Object.keys(changes).length && !confirm('Discard the changes you have not saved?')) return
    record = await run(isMonsters ? 'monster_record' : 'table_record', args({ id }))
    changes = {}
  }
  async function save() {
    const r = await run(isMonsters ? 'monster_save' : 'table_save', args({ id: record.id, values: changes }))
    record = r.record
    changes = {}
    toast(`Saved into “${app.status.mods.find((m: Json) => m.id === app.status.active_mod)?.name ?? 'your mod'}”`)
    await loadRows()
  }
  // The page is re-created per editor ({#key}), so this runs once.
  loadRows()
  const title = $derived(rows.find((r) => r.id === record?.id)?.label ?? '')
  const dirty = $derived(Object.keys(changes).length)
</script>

<div class="split">
  <SearchList {rows} bind:selected onselect={(r) => open(r.id)} placeholder={`Search ${editor.title.toLowerCase()}…`} />
  <div class="detail-pane">
    {#if record}
      <div class="toolbar">
        <div class="grow">
          <h2 style="margin: 0">{title}</h2>
          <span class="muted">{editor.title} · #{record.id}</span>
        </div>
        {#if dirty}<button class="ghost" onclick={() => (changes = {})}><Icon name="undo" />Undo</button>{/if}
        <button class="primary" disabled={!dirty} onclick={save}><Icon name="save" />Save{dirty ? ` (${dirty})` : ''}</button>
      </div>
      <FieldForm fields={record.fields} values={record.values} options={record.options} bind:changes />
    {:else}
      <div class="empty">
        <Icon name={editor.icon} size={40} />
        <p>{editor.about}</p>
        <p>Pick an entry on the left to edit it. Changes go into your active mod; the game's own files are never changed.</p>
      </div>
    {/if}
  </div>
</div>
