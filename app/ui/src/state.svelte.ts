// App-wide state: the status from the core, the current page, messages.
import { api, type Json } from './api'

export const app = $state({
  status: null as Json,
  editors: [] as Json[],
  page: 'home' as string,
  toasts: [] as { id: number; text: string; kind: 'ok' | 'error' }[],
  busy: 0,
})

let toastId = 0
export function toast(text: string, kind: 'ok' | 'error' = 'ok') {
  const id = ++toastId
  app.toasts.push({ id, text, kind })
  setTimeout(() => {
    const i = app.toasts.findIndex((t) => t.id === id)
    if (i >= 0) app.toasts.splice(i, 1)
  }, kind === 'error' ? 8000 : 3500)
}

/** Runs a command, counting it as busy and showing errors. */
export async function run(cmd: string, args: Record<string, Json> = {}): Promise<Json> {
  app.busy++
  try {
    return await api(cmd, args)
  } catch (e) {
    toast(String((e as Error).message ?? e), 'error')
    throw e
  } finally {
    app.busy--
  }
}

export async function refresh() {
  app.status = await run('status')
  if (app.editors.length === 0 && app.status?.game_ok) app.editors = await run('editors')
}
