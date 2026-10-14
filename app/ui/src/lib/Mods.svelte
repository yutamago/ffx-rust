<script lang="ts">
  import { app, run, refresh, toast } from '../state.svelte'
  import type { Json } from '../api'
  import Icon from './Icon.svelte'

  let name = $state('')
  let files = $state<Record<string, string[]>>({})
  async function create() {
    app.status = await run('mod_create', { name })
    name = ''
    toast('Mod created; new changes go into it')
  }
  async function toggle(m: Json) {
    app.status = await run('mod_update', { id: m.id, enabled: !m.enabled })
  }
  async function rename(m: Json, value: string) {
    if (value && value !== m.name) app.status = await run('mod_update', { id: m.id, name: value })
  }
  async function select(m: Json) {
    app.status = await run('mod_select', { id: m.id })
  }
  async function showFiles(m: Json) {
    if (files[m.id]) delete files[m.id]
    else files[m.id] = await run('mod_files', { id: m.id })
  }
  async function remove(m: Json, path: string) {
    if (!confirm(`Take ${path} out of “${m.name}”? The game's own file is used again.`)) return
    files[m.id] = await run('mod_remove_file', { id: m.id, path })
    refresh()
  }
</script>

<div class="detail-pane" style="height: 100%">
  <h1>Mods</h1>
  <p class="muted">The game applies switched-on mods in this order; a later mod wins where two change the same file. Your edits go into the <b>active</b> mod.</p>
  <div class="card" style="margin-bottom: 16px">
    <div class="row">
      <input placeholder="Name of a new mod" bind:value={name} onkeydown={(e) => e.key === 'Enter' && name && create()} />
      <button class="primary" disabled={!name} onclick={create}><Icon name="plus" />New mod</button>
    </div>
  </div>
  {#each app.status?.mods ?? [] as m (m.id)}
    <div class="card" style="margin-bottom: 10px">
      <div class="row">
        <label class="switch" title={m.enabled ? 'Switched on' : 'Switched off'}>
          <input type="checkbox" checked={m.enabled} onchange={() => toggle(m)} />
        </label>
        <div class="grow">
          <input value={m.name} onchange={(e) => rename(m, e.currentTarget.value)} style="font-weight: 600; background: transparent; border-color: transparent; padding-left: 0" aria-label="Mod name" />
          <span class="muted mono" style="font-size: 12px">mods/{m.id} · {m.files} file{m.files === 1 ? '' : 's'}</span>
        </div>
        {#if app.status.active_mod === m.id}<span class="chip on"><Icon name="check" size={14} />Active</span>
        {:else}<button onclick={() => select(m)}>Make active</button>{/if}
        <button class="ghost" onclick={() => showFiles(m)}>{files[m.id] ? 'Hide files' : 'Files'}</button>
      </div>
      {#if files[m.id]}
        <div style="margin-top: 10px">
          {#each files[m.id] as f}
            <div class="row" style="padding: 3px 0"><span class="grow mono" style="font-size: 12px">{f}</span><button class="ghost danger" onclick={() => remove(m, f)}><Icon name="trash" size={15} /></button></div>
          {:else}<p class="muted">No files yet.</p>{/each}
        </div>
      {/if}
    </div>
  {:else}
    <p class="empty">No mods yet. Create one above, or just start editing: a first mod is made for you.</p>
  {/each}
</div>
