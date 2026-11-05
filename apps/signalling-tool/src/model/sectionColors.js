/**
 * The colours the track sections are told apart by on the plan: ten, and no
 * section the colour of one beside it — one it shares a detection point with.
 * The sections with the most neighbours are coloured first, each the colour
 * free among its neighbours' that is used least so far. The speed ranges are
 * coloured the same way (apartColors).
 */
export const SECTION_COLORS = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd', '#8c564b', '#e377c2', '#17becf', '#bcbd22', '#0a1f8f']

/** Each of `ids` a colour none of its neighbours (`near`: Map id → Set of ids) has, where the colours suffice: Map id → colour. */
export function apartColors(ids, near, colors = SECTION_COLORS) {
  const order = [...ids].sort((a, b) => (near.get(b)?.size ?? 0) - (near.get(a)?.size ?? 0))
  const used = colors.map(() => 0)
  const out = new Map()
  for (const id of order) {
    const taken = new Set([...(near.get(id) ?? [])].map(n => out.get(n)).filter(Boolean))
    const free = colors.map((c, i) => i).filter(i => !taken.has(colors[i]))
    // With more neighbours than colours, the one least used among them.
    const pick = (free.length ? free : colors.map((c, i) => i)).sort((a, b) => used[a] - used[b] || a - b)[0]
    used[pick]++
    out.set(id, colors[pick])
  }
  return out
}

/** Each section's colour: Map id → colour. */
export function sectionColors(sections, colors = SECTION_COLORS) {
  const byPoint = new Map()
  for (const s of sections) for (const p of s.boundedBy ?? []) byPoint.set(p, [...(byPoint.get(p) ?? []), s.id])
  const near = new Map(sections.map(s => [s.id, new Set((s.boundedBy ?? []).flatMap(p => byPoint.get(p)).filter(id => id !== s.id))]))
  return apartColors(sections.map(s => s.id), near, colors)
}
