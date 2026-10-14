// The app's commands: in the window (Tauri) through `invoke`, in a browser
// through the development server's `/api`.

export type Json = any

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

export async function api(cmd: string, args: Record<string, Json> = {}): Promise<Json> {
  if (window.__TAURI_INTERNALS__) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('call', { cmd, args })
  }
  const res = await fetch('/api', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cmd, args }),
  })
  const answer = await res.json()
  if ('error' in answer) throw new Error(answer.error)
  return answer.ok
}

/** A chosen file as base64, for commands that take `data`. */
export function fileData(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const url = reader.result as string
      resolve(url.slice(url.indexOf(',') + 1))
    }
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(file)
  })
}
