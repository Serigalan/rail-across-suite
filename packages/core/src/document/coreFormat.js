import { derivedId } from './canonical.js'

// What of the record and its core 1.0 document the page still works out
// itself: the construction stages a project has without its own, and the
// operational point register its tracks' and platforms' station texts make
// (R10) — which point each object names, by the object itself. Turning a
// record into a document and back is the door's (crates/doc; decision 0020,
// step 10 took that JavaScript out).

/** The domains this build reads and writes, with their versions. */
export const DOMAIN_VERSIONS = { core: '1.0', layout: '1.0', signalling: '1.0' }

/** An object without its null members. */
const withoutNull = (obj) => Object.fromEntries(Object.entries(obj).filter(([, v]) => v != null))

/** A text, or nothing where it is empty. */
const text = (v) => (v == null || String(v).trim() === '' ? undefined : v)

// ── Construction stages ─────────────────────────────────────────────────────

/** The two stages of a project that has none of its own: existing and final, ids drawn from the project's. */
export const defaultStages = (projectId) => [
  { id: derivedId(`${projectId}\u0000stage\u0000existing`) },
  { id: derivedId(`${projectId}\u0000stage\u0000final`) },
]

// ── Operational points ──────────────────────────────────────────────────────

/**
 * The register the tracks' and platforms' station texts make, with the points
 * the record keeps of it (`stored`), and which point each of them names:
 * { points, pointOf(obj) }. One point per station name, holding the codes
 * stated for it; where two objects state different codes under one name,
 * each code its own point. A kept point is the one of its name: it keeps its
 * id and its codes, and takes the codes the objects agree on where it has
 * none; an object stating another code is a point of its own.
 */
export function operationalRegister(projectId, tracks, platforms, stored = []) {
  const named = [
    ...tracks.map(t => ({ obj: t, name: text(t.stationName), uic: text(t.uicStation), rl100: undefined })),
    ...platforms.map(p => ({ obj: p, name: text(p.stationName), uic: undefined, rl100: text(p.code) })),
  ].filter(e => e.name !== undefined || e.uic !== undefined || e.rl100 !== undefined)
  const keyOf = (e) => (e.name !== undefined ? `name\u0000${e.name}` : `code\u0000${e.uic ?? ''}\u0000${e.rl100 ?? ''}`)
  const groups = new Map()
  const kept = new Map()
  const points = new Map()
  for (const p of stored) {
    const key = keyOf({ name: text(p.name), uic: text(p.uic), rl100: text(p.rl100) })
    // A second point kept under one name is named by nothing; it stays as it is.
    if (kept.has(key)) points.set(`id\u0000${p.id}`, p)
    else { kept.set(key, p); groups.set(key, []) }
  }
  for (const e of named) {
    const key = keyOf(e)
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key).push(e)
  }
  const of = new Map()
  const derived = (own, e) => withoutNull({ id: derivedId(`${projectId}\u0000operationalPoint\u0000${own}`), name: e.name, rl100: e.rl100, uic: e.uic })
  for (const [key, entries] of groups) {
    const distinct = (k) => new Set(entries.map(e => e[k]).filter(v => v !== undefined).map(String))
    const keep = kept.get(key)
    if (keep) {
      const agreed = (k) => (distinct(k).size === 1 ? [...distinct(k)][0] : undefined)
      const point = withoutNull({ id: keep.id, name: keep.name, rl100: keep.rl100 ?? agreed('rl100'), uic: keep.uic ?? agreed('uic') })
      points.set(key, point)
      for (const e of entries) {
        const fits = ['rl100', 'uic'].every(k => e[k] === undefined || String(e[k]) === String(point[k]))
        if (fits) { of.set(e.obj, point.id); continue }
        const own = `${key}\u0000${e.uic ?? ''}\u0000${e.rl100 ?? ''}`
        if (!points.has(own)) points.set(own, derived(own, e))
        of.set(e.obj, points.get(own).id)
      }
      continue
    }
    const agree = distinct('uic').size <= 1 && distinct('rl100').size <= 1
    for (const e of entries) {
      const own = agree ? key : `${key}\u0000${e.uic ?? ''}\u0000${e.rl100 ?? ''}`
      if (!points.has(own)) {
        const from = agree ? entries : [e]
        points.set(own, derived(own, {
          name: e.name,
          rl100: from.find(x => x.rl100 !== undefined)?.rl100,
          uic: from.find(x => x.uic !== undefined)?.uic,
        }))
      }
      of.set(e.obj, points.get(own).id)
    }
  }
  // A point kept under a name that the names make anew is the one they make.
  const unique = new Map([...points.values()].map(p => [p.id, p]))
  return { points: [...unique.values()], pointOf: (obj) => of.get(obj) }
}
