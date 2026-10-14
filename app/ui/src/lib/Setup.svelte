<script lang="ts">
  import { app, run, refresh } from '../state.svelte'
  import Icon from './Icon.svelte'
  let path = $state(app.status?.game_dir ?? '')
  let error = $state('')
  async function use() {
    error = ''
    try {
      app.status = await run('set_game_dir', { path })
      await refresh()
    } catch (e) {
      error = String((e as Error).message ?? e)
    }
  }
</script>

<div style="display: grid; place-items: center; height: 100%">
  <div class="card" style="width: 560px; padding: 28px">
    <h1><Icon name="folder" size={22} /> Where is the game?</h1>
    <p class="muted">Choose the folder of <b>FINAL FANTASY X/X-2 HD Remaster</b>: the one with <span class="mono">FFX.exe</span>. In Steam: right-click the game › Manage › Browse local files, and copy the address.</p>
    <input class="mono" placeholder="C:\Program Files (x86)\Steam\steamapps\common\FINAL FANTASY FFX&FFX-2 HD Remaster" bind:value={path} onkeydown={(e) => e.key === 'Enter' && use()} />
    {#if error}<p style="color: var(--danger)">{error}</p>{/if}
    <div class="row" style="margin-top: 14px; justify-content: flex-end"><button class="primary" disabled={!path} onclick={use}>Use this folder</button></div>
  </div>
</div>
