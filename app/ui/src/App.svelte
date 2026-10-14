<script lang="ts">
  import { app, refresh, run } from './state.svelte'
  import type { Json } from './api'
  import Icon from './lib/Icon.svelte'
  import Home from './lib/Home.svelte'
  import Mods from './lib/Mods.svelte'
  import Setup from './lib/Setup.svelte'
  import RecordEditor from './lib/RecordEditor.svelte'
  import Encounters from './lib/Encounters.svelte'
  import Dialogue from './lib/Dialogue.svelte'
  import Models from './lib/Models.svelte'
  import Scripts from './lib/Scripts.svelte'
  import Files from './lib/Files.svelte'

  refresh()
  const visible = $derived(app.editors.filter((e: Json) => e.simple || app.status?.advanced))
  const groups = $derived.by(() => {
    const out: { name: string; editors: Json[] }[] = []
    for (const e of visible) {
      let g = out.find((x) => x.name === e.group)
      if (!g) out.push((g = { name: e.group, editors: [] }))
      g.editors.push(e)
    }
    return out
  })
  const editor = $derived(app.editors.find((e: Json) => e.id === app.page))
  const activeMod = $derived(app.status?.mods?.find((m: Json) => m.id === app.status?.active_mod))

  async function setLanguage(code: string) {
    app.status = await run('set_language', { language: code })
    app.editors = []
    await refresh()
    const page = app.page
    app.page = 'home'
    queueMicrotask(() => (app.page = page))
  }
  async function setAdvanced(on: boolean) {
    app.status = await run('set_advanced', { on })
  }
  async function selectMod(id: string) {
    if (id === '__new') app.page = 'mods'
    else app.status = await run('mod_select', { id })
  }
</script>

{#if !app.status}
  <div class="empty" style="padding-top: 30vh">Starting…</div>
{:else if !app.status.game_ok}
  <Setup />
{:else}
  <div class="shell">
    <aside>
      <div class="brand"><span class="logo">X</span><span><b>FFX Studio</b><span class="muted" style="display: block; font-size: 11px">mods and launcher</span></span></div>
      <nav>
        <button class:active={app.page === 'home'} onclick={() => (app.page = 'home')}><Icon name="home" />Home</button>
        <button class:active={app.page === 'mods'} onclick={() => (app.page = 'mods')}><Icon name="mods" />Mods</button>
        {#each groups as g (g.name)}
          <div class="nav-group">{g.name}</div>
          {#each g.editors as e (e.id)}
            <button class:active={app.page === e.id} onclick={() => (app.page = e.id)}><Icon name={e.icon} />{e.title}</button>
          {/each}
        {/each}
      </nav>
      <div class="side-foot">
        <label for="lang">Text language</label>
        <select id="lang" value={app.status.language} onchange={(e) => setLanguage(e.currentTarget.value)}>
          {#each app.status.languages as l (l.code)}<option value={l.code}>{l.name}</option>{/each}
        </select>
        <label class="row" style="margin-top: 10px; color: var(--text)">
          <input type="checkbox" checked={app.status.advanced} onchange={(e) => setAdvanced(e.currentTarget.checked)} /> Advanced mode
        </label>
      </div>
    </aside>
    <main>
      <header>
        <span class="muted">Editing</span>
        <select value={app.status.active_mod ?? ''} onchange={(e) => selectMod(e.currentTarget.value)} style="width: 240px" aria-label="Active mod">
          {#if !app.status.active_mod}<option value="">(a new mod, made on first save)</option>{/if}
          {#each app.status.mods as m (m.id)}<option value={m.id}>{m.name}{m.enabled ? '' : ' (off)'}</option>{/each}
          <option value="__new">+ New mod…</option>
        </select>
        {#if activeMod && !activeMod.enabled}<span class="chip" style="color: var(--gold)"><Icon name="warn" size={14} />switched off: the game will not load it</span>{/if}
        <span class="grow"></span>
        {#if app.busy}<span class="muted">Working…</span>{/if}
        <span class="muted mono" style="font-size: 12px" title="Game folder">{app.status.game_dir}</span>
      </header>
      <section>
        {#if app.page === 'home'}<Home />
        {:else if app.page === 'mods'}<Mods />
        {:else if editor?.kind === 'table' || editor?.kind === 'monsters'}{#key editor.id}<RecordEditor {editor} />{/key}
        {:else if editor?.kind === 'encounters'}<Encounters />
        {:else if editor?.kind === 'dialogue'}{#key app.status.language}<Dialogue />{/key}
        {:else if editor?.kind === 'models'}<Models />
        {:else if editor?.kind === 'scripts'}<Scripts />
        {:else if editor?.kind === 'files'}<Files />
        {/if}
      </section>
    </main>
  </div>
{/if}

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}<div class="toast" class:error={t.kind === 'error'}>{t.text}</div>{/each}
</div>

<style>
  .shell { display: grid; grid-template-columns: 230px 1fr; height: 100vh; }
  aside { background: #0d1628; border-right: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; }
  .brand { display: flex; align-items: center; gap: 10px; padding: 16px; border-bottom: 1px solid var(--line); }
  .logo { width: 34px; height: 34px; border-radius: 9px; display: grid; place-items: center; font-weight: 800; color: #2a1d00; background: linear-gradient(180deg, #f3d27e, #c99a36); }
  nav { flex: 1; overflow: auto; padding: 8px; display: flex; flex-direction: column; gap: 2px; }
  nav button { background: transparent; border: none; justify-content: flex-start; padding: 7px 10px; color: var(--muted); }
  nav button:hover { color: var(--text); background: var(--panel); }
  nav button.active { color: var(--text); background: var(--panel-2); }
  .nav-group { font-size: 11px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; padding: 12px 10px 4px; }
  .side-foot { padding: 14px; border-top: 1px solid var(--line); }
  main { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  header { display: flex; align-items: center; gap: 10px; padding: 10px 18px; border-bottom: 1px solid var(--line); background: #0d1628; }
  section { flex: 1; min-height: 0; overflow: hidden; }
  .toasts { position: fixed; bottom: 18px; right: 18px; display: flex; flex-direction: column; gap: 8px; z-index: 50; }
  .toast { background: var(--panel-2); border: 1px solid var(--ok); border-left-width: 4px; padding: 10px 14px; border-radius: 8px; max-width: 420px; box-shadow: 0 6px 20px #0008; }
  .toast.error { border-color: var(--danger); }
</style>
