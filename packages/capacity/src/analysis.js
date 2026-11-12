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
 * rate for `line`, and `input`, the trains as they run there (their blocks
 * with where each lies on the stretch, to draw them).
 */
export function analyseStretch(stretch, trains, runs, { from, to, additional = {}, line = 'mixed' } = {}) {
  const input = trainsOnStretch(stretch, trains, runs)
  const compressed = callNative('capacity.compress', { trains: input, from, to, additional })
  const inPeriod = input.filter(t => t.at >= from && t.at < to)
  const headways = headwayTable(callNative('capacity.headways', { trains: inPeriod }))
  return { ...compressed, headways, bound: boundOf(line, to - from), input: inPeriod }
}

/** How long each track and track section of a station is held over the period [from, to): [{ resource, held, rate, trains }]. */
export function stationOccupancy(station, trains, runs, record, { from, to }) {
  return callNative('capacity.resourceOccupancy', { trains: trainsAtStation(station, trains, runs, record), from, to })
}
