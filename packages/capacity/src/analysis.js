import { callNative } from '@rail-across/core/native'
import { trainsAtStation, trainsOnStretch } from './stretches.js'

/**
 * A stretch's or a station's capacity from the trains' blocking times, as the
 * core's Rust module computes it (crates/capacity, K1).
 */

/** The recommended bounds of the occupancy rate after UIC Code 406, by kind of line: { line, peak, day }. */
export const BOUNDS = [
  { line: 'suburban', peak: 0.85, day: 0.70 },
  { line: 'highSpeed', peak: 0.75, day: 0.60 },
  { line: 'mixed', peak: 0.75, day: 0.60 },
]

/** The bound for a kind of line and a period: the peak hour's for a period of up to two hours, else the day's. */
export function boundOf(line, period) {
  const b = BOUNDS.find(x => x.line === line) ?? BOUNDS.find(x => x.line === 'mixed')
  return period <= 2 * 3600 ? b.peak : b.day
}

/**
 * The minimum headways of a stretch by kind: [{ before, after, least, most, pairs }] —
 * for each kind behind each, the least and the largest minimum headway of
 * two such trains [s], and of how many pairs.
 */
export function headwayTable(headways) {
  const by = new Map()
  for (const h of headways) {
    const key = `${h.kinds[0]}\u0000${h.kinds[1]}`
    const row = by.get(key) ?? { before: h.kinds[0], after: h.kinds[1], least: Infinity, most: -Infinity, pairs: 0 }
    row.least = Math.min(row.least, h.least)
    row.most = Math.max(row.most, h.least)
    row.pairs++
    by.set(key, row)
  }
  return [...by.values()].sort((a, b) => a.before.localeCompare(b.before) || a.after.localeCompare(b.after))
}

/**
 * A stretch compressed after UIC Code 406 over the period [from, to): what
 * the crate gives (the trains compressed, the occupancy time, the rates, the
 * trains that set it) with `headways` by kind, `bound`, the recommended
 * rate for `line`, `input`, the trains as they run there (their blocks
 * with where each lies on the stretch, to draw them), `areas`, what two or
 * more of them hold (conflictAreas), and `binding`, the elements that bind the
 * critical chain (bindingElements).
 */
export function analyseStretch(stretch, trains, runs, { from, to, additional = {}, line = 'mixed' } = {}) {
  const input = trainsOnStretch(stretch, trains, runs)
  const compressed = callNative('capacity.compress', { trains: input, from, to, additional })
  const inPeriod = input.filter(t => t.at >= from && t.at < to)
  const headways = headwayTable(callNative('capacity.headways', { trains: inPeriod }))
  const areas = callNative('capacity.conflictAreas', { trains: inPeriod, from, to })
  return { ...compressed, headways, bound: boundOf(line, to - from), input: inPeriod, areas, binding: bindingElements(compressed, inPeriod) }
}

/**
 * The elements that bind the critical chain of a compressed stretch
 * (Belegungselemente): for each train of it after the first, [{ before,
 * after, resource, block, beforeBlock, at }] — the resource where its blocking
 * time touches the one's before (a key of the crate: `section:<id>`,
 * `track:<id>:<lo>-<hi>`), the block of each it is held by ({ signalId,
 * targetId }), the moment as compressed [s].
 */
export function bindingElements(compressed, input) {
  const byId = new Map(compressed.trains.map(t => [t.id, t]))
  const blocksOf = new Map(input.map(t => [t.id, t.blocks]))
  const blockOf = (id, i) => {
    const b = blocksOf.get(id)?.[i]
    return b ? { signalId: b.signalId ?? null, targetId: b.targetId ?? null } : null
  }
  return (compressed.critical ?? []).slice(1).map(id => byId.get(id)).filter(t => t?.binding && t.boundBy).map(t => ({
    before: t.boundBy, after: t.id, resource: t.binding.resource, at: t.binding.taken,
    block: blockOf(t.id, t.binding.block), beforeBlock: blockOf(t.boundBy, t.binding.beforeBlock),
  }))
}

/** How long each track and track section of a station is held over the period [from, to): [{ resource, held, rate, trains }]. */
export function stationOccupancy(station, trains, runs, record, { from, to }) {
  return callNative('capacity.resourceOccupancy', { trains: trainsAtStation(station, trains, runs, record), from, to })
}
