<script lang="ts">
  import { app, run, refresh, toast } from '../state.svelte'
  import type { Json } from '../api'
  import Icon from './Icon.svelte'

  const enabled = $derived((app.status?.mods ?? []).filter((m: Json) => m.enabled))
  const simple = $derived(app.editors.filter((e: Json) => e.simple))
  async function play(mods: boolean) {
    try {
      await run('launch', { mods })
      toast(mods ? 'Starting the game with your mods…' : 'Starting the original game…')
      setTimeout(refresh, 4000)
    } catch {
      /* shown as a toast */
    }
  }
</script>

<div class="detail-pane" style="height: 100%">
  <div class="card hero">
    <div class="grow">
      <h1 style="font-size: 26px">Final Fantasy X</h1>
      <p class="muted" style="margin: 0">
        {enabled.length ? `${enabled.length} mod${enabled.length === 1 ? '' : 's'} switched on: ${enabled.map((m: Json) => m.name).join(', ')}` : 'No mods switched on'}
      </p>
    </div>
    <button class="gold" style="font-size: 17px; padding: 12px 26px" onclick={() => play(true)} disabled={app.status?.running}>
      <Icon name="play" size={20} />{app.status?.running ? 'Running' : 'Play'}
    </button>
    <button onclick={() => play(false)} disabled={app.status?.running} title="Without any mods">Play original</button>
  </div>

  <h3 style="margin-top: 26px">Make a change</h3>
  <div class="tiles">
    {#each simple as e (e.id)}
      <button class="tile" onclick={() => (app.page = e.id)}>
        <span class="tile-icon"><Icon name={e.icon} size={22} /></span>
        <span><b>{e.title}</b><span class="muted" style="display: block; font-size: 12px; margin-top: 2px">{e.about}</span></span>
      </button>
    {/each}
  </div>

  <h3 style="margin-top: 26px">How it works</h3>
  <div class="card muted" style="line-height: 1.6">
    Every change you make is saved into your <b style="color: var(--text)">active mod</b>, a folder in the game's
    <span class="mono">mods</span> folder; the game's own files stay untouched. <b style="color: var(--text)">Play</b> starts the game
    with all switched-on mods; <b style="color: var(--text)">Play original</b> without them. Switch mods on and off, or start a new one,
    under <a href="#mods" onclick={() => (app.page = 'mods')}>Mods</a>.
  </div>
</div>

<style>
  .hero { display: flex; align-items: center; gap: 16px; padding: 22px; background: linear-gradient(120deg, #13233f, #121b2e 60%); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .tile { text-align: left; align-items: flex-start; padding: 14px; background: var(--panel); gap: 12px; }
  .tile-icon { color: var(--accent); background: #0e1a30; border-radius: 8px; padding: 8px; display: inline-flex; }
</style>
