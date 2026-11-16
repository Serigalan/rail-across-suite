import { extentsOf } from '@rail-across/timetables'

/**
 * What the schematic plan of a stretch marks (decision 0030, point 1), as
 * stretches of track: the conflict areas the trains of the period both hold
 * and the elements that bind the critical chain — pure functions of the
 * analysis and the record.
 */

/**
 * Where a resource of the capacity crate lies: [{ trackId, lo, hi }] — a
 * track section over the parts its detection points bound, a stretch of a
 * track as it is named (`track:<id>:<lo>-<hi>`).
 */
export function resourceStretches(key, record) {
  const [kind, ...rest] = String(key).split(':')
  if (kind === 'section') {
    const parts = extentsOf(record).get(rest.join(':'))?.parts ?? []
    return parts.map(p => ({ trackId: p.trackId, lo: Math.min(p.from, p.to), hi: Math.max(p.from, p.to) }))
  }
  if (kind !== 'track') return []
  const range = rest.at(-1) ?? ''
  const trackId = rest.slice(0, -1).join(':')
  const [lo, hi] = range.split('-').map(Number)
  return trackId && Number.isFinite(lo) && Number.isFinite(hi) ? [{ trackId, lo: Math.min(lo, hi), hi: Math.max(lo, hi) }] : []
}

/**
 * The marks of a stretch's plan: { areas: [{ key, trains, held, stretches }],
 * binding: [{ n, key, before, after, at, block, stretches }] } — `trains` how
 * many hold an area; `n` a binding element's number in the critical chain,
 * from 1, the same element bound twice keeping the number it was first given.
 */
export function planMarks(analysis, record) {
  if (!analysis) return { areas: [], binding: [] }
  const areas = (analysis.areas ?? []).map(a => ({ key: a.resource, trains: a.trains.length, held: a.held, stretches: resourceStretches(a.resource, record) }))
  const numbers = new Map()
  const binding = (analysis.binding ?? []).map(b => {
    if (!numbers.has(b.resource)) numbers.set(b.resource, numbers.size + 1)
    return { ...b, n: numbers.get(b.resource), key: b.resource, stretches: resourceStretches(b.resource, record) }
  })
  return { areas, binding }
}

/** The colour of a conflict area by how many trains hold it. */
export const areaColor = (trains) => (trains >= 4 ? '#c0392b' : trains === 3 ? '#e67e22' : '#f1c40f')

/** The colour the binding elements are drawn in. */
export const BINDING_COLOR = '#6f2dbd'
