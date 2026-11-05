import { signallingSpeedOf } from '@rail-across/core/signalling'
import { apartColors } from './sectionColors.js'
import { cut, isSpeedSignal, routeSpeedSections, speedSignalSpeed } from './routeSpeeds.js'
import { routeStretches } from './routeSearch.js'

/**
 * The speed ranges: where the signals signal a speed, as a train runs a
 * route — from its start signal at the speed it states for the route (the
 * Zs 3 there) up to the first Lf 7 on it, from each Lf 7 at what it shows up
 * to the next or the target signal. A range is one signal's at one speed:
 * the stretches of every route it holds over, joined. Two ranges are
 * neighbours where one follows the other on a route, or a route on the one
 * the other starts at its target, or where they share track; each has a
 * colour none of its neighbours has. This is what the Fahrplan-Sim runs at.
 */

/** Where along a track two parts touch or overlap to count as neighbours [m]. */
const TOUCH = 1

/** Stretches as parts of track, low station first: [{ trackId, from, to }]. */
const partsOf = (stretches) => stretches.filter(s => s.hi - s.lo > 0.5).map(s => ({ trackId: s.trackId, from: s.lo, to: s.hi }))

/** Parts of track joined where they overlap or touch, by track and station. */
function joined(parts) {
  const out = []
  const sorted = [...parts].sort((a, b) => a.trackId.localeCompare(b.trackId) || a.from - b.from)
  for (const p of sorted) {
    const last = out[out.length - 1]
    if (last && last.trackId === p.trackId && p.from <= last.to + 0.01) last.to = Math.max(last.to, p.to)
    else out.push({ ...p })
  }
  return out
}

/**
 * The speed ranges of a domain: [{ id, signal, speed, board, routeIds,
 * parts, length, color }] — `board` whether it starts at an Lf 7, `parts`
 * the track it covers, `length` theirs [m].
 */
export function speedRanges(domain, net) {
  const { routes, signals } = domain.collections
  const byKey = new Map()
  const near = new Map()
  const link = (a, b) => {
    if (!a || !b || a === b) return
    near.set(a, (near.get(a) ?? new Set()).add(b))
    near.set(b, (near.get(b) ?? new Set()).add(a))
  }
  const firstOf = new Map()   // start signal id → keys of the ranges its routes start with
  const lastOf = []   // [{ target signal id, key }]
  for (const route of routes) {
    if (route.kind !== 'train') continue
    const speed = signallingSpeedOf(domain, route)
    const stretches = routeStretches(net, route, signals)
    if (!stretches.length) continue
    const keys = []
    for (const [i, section] of routeSpeedSections(net, route, signals).entries()) {
      const board = i > 0 && isSpeedSignal(section.signal)
      const v = board ? speedSignalSpeed(section.signal) : speed
      if (!(v > 0) || !section.signal) { keys.push(null); continue }
      const key = `${section.signal.id}|${v}`
      if (!byKey.has(key)) byKey.set(key, { id: key, signal: section.signal, speed: v, board, routeIds: [], parts: [] })
      const range = byKey.get(key)
      if (!range.routeIds.includes(route.id)) range.routeIds.push(route.id)
      range.parts.push(...partsOf(cut(stretches, section.from, section.to)))
      keys.push(key)
    }
    keys.forEach((k, i) => link(k, keys[i + 1]))
    if (keys[0]) firstOf.set(route.startSignalId, [...(firstOf.get(route.startSignalId) ?? []), keys[0]])
    if (keys.at(-1)) lastOf.push({ signalId: route.targetSignalId, key: keys.at(-1) })
  }
  for (const { signalId, key } of lastOf) for (const k of firstOf.get(signalId) ?? []) link(key, k)
  const ranges = [...byKey.values()].map(r => {
    const parts = joined(r.parts)
    return { ...r, parts, length: parts.reduce((sum, p) => sum + p.to - p.from, 0) }
  })
  for (const a of ranges) {
    for (const b of ranges) {
      if (a.id < b.id && a.parts.some(p => b.parts.some(q => p.trackId === q.trackId && p.from <= q.to + TOUCH && q.from <= p.to + TOUCH))) link(a.id, b.id)
    }
  }
  const colors = apartColors(ranges.map(r => r.id), near)
  return ranges.map(r => ({ ...r, color: colors.get(r.id) }))
}
