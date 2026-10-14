<script lang="ts">
  // Fields of a record, edited in place. `changes` collects what differs
  // from `values`.
  import type { Json } from '../api'
  import { app } from '../state.svelte'
  let {
    fields,
    values,
    options = {},
    changes = $bindable({}),
  }: { fields: Json[]; values: Json; options?: Json; changes?: Record<string, Json> } = $props()

  const shown = $derived(fields.filter((f) => app.status?.advanced || !f.advanced))
  const groups = $derived.by(() => {
    const out: { name: string; fields: Json[] }[] = []
    for (const f of shown) {
      const name = f.group ?? (f.editor.type === 'text' || f.editor.type === 'textarea' ? 'Text' : 'Values')
      let g = out.find((x) => x.name === name)
      if (!g) out.push((g = { name, fields: [] }))
      g.fields.push(f)
    }
    return out
  })

  // Outside advanced mode, notes lose their references into FFX.exe.
  function doc(text: string): string {
    if (app.status?.advanced) return text
    return text
      .replace(/\s*\([^()]*(0x|[A-Z][a-z]+_[A-Z])[^()]*\)/g, '')
      .replace(/(^|[;,]\s*)actor \+0x[0-9a-f]+(\/\+0x[0-9a-f]+)?/gi, '$1')
      .replace(/\b[A-Z][A-Za-z]+_[A-Za-z]+ 0x[0-9a-f]{8}/g, '')
      .replace(/^[\s;,]+|[\s;,]+$/g, '')
      .replace(/\s+([;,.])/g, '$1')
      .replace(/;\s*;/g, ';')
  }

  function current(key: string) {
    return key in changes ? changes[key] : values[key]
  }
  function set(key: string, v: Json) {
    if (v === values[key]) delete changes[key]
    else changes[key] = v
  }
  function setInt(f: Json, text: string) {
    const v = Number(text)
    if (!Number.isInteger(v)) return
    set(f.key, Math.max(f.editor.min ?? 0, Math.min(f.editor.max ?? 0xffffffff, v)))
  }
  function refOptions(f: Json): Json[] {
    const list = (options[f.editor.ref] ?? []) as Json[]
    const v = current(f.key)
    return list.some((o) => o.value === v) ? list : [{ value: v, label: `${v} (other)` }, ...list]
  }
  function treasureOptions(): Json[] {
    const kind = current('kind')
    const list = kind === 2 ? options.item : kind === 10 ? options.key_item : null
    return list ?? []
  }
</script>

{#each groups as g (g.name)}
  <h3 style="margin-top: 18px">{g.name}</h3>
  <div class="fields">
    {#each g.fields as f (f.key)}
      {@const e = f.editor}
      <div class="field" class:wide={e.type === 'textarea' || e.type === 'hex'} class:changed={f.key in changes}>
        <label for={`f-${f.key}`}>{f.label}</label>
        {#if e.type === 'text'}
          <input id={`f-${f.key}`} value={current(f.key) ?? ''} oninput={(ev) => set(f.key, ev.currentTarget.value)} />
        {:else if e.type === 'textarea'}
          <textarea id={`f-${f.key}`} rows="3" value={current(f.key) ?? ''} oninput={(ev) => set(f.key, ev.currentTarget.value)}></textarea>
        {:else if e.type === 'choice'}
          <select id={`f-${f.key}`} value={current(f.key)} onchange={(ev) => set(f.key, Number(ev.currentTarget.value))}>
            {#each e.choices as [v, l]}<option value={v}>{l}</option>{/each}
            {#if !e.choices.some(([v]: [number]) => v === current(f.key))}<option value={current(f.key)}>{current(f.key)} (other)</option>{/if}
          </select>
        {:else if e.type === 'ref'}
          <select id={`f-${f.key}`} value={current(f.key)} onchange={(ev) => set(f.key, Number(ev.currentTarget.value))}>
            {#each refOptions(f) as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
          </select>
        {:else if e.type === 'treasure' && treasureOptions().length > 0}
          <select id={`f-${f.key}`} value={current(f.key)} onchange={(ev) => set(f.key, Number(ev.currentTarget.value))}>
            {#each treasureOptions() as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
          </select>
        {:else if e.type === 'hex'}
          <input id={`f-${f.key}`} class="mono" value={current(f.key) ?? ''} oninput={(ev) => set(f.key, ev.currentTarget.value)} />
        {:else}
          <input id={`f-${f.key}`} type="number" min={e.min} max={e.max} value={current(f.key)} oninput={(ev) => setInt(f, ev.currentTarget.value)} />
        {/if}
        {#if f.doc && doc(f.doc)}<span class="doc">{doc(f.doc)}</span>{/if}
      </div>
    {/each}
  </div>
{/each}
