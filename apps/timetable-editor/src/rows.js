import { formatClock } from '@rail-across/timetables'

/**
 * What the editor shows of trains and their runs: the order of the list, a
 * train's line under its title, where a stop asked for at a station passed
 * goes among its points, the axis and the lines of the graphical timetable.
 */

/** Trains by their departure, then by number. */
export const byDeparture = (trains) => [...trains].sort((a, b) => a.departure - b.departure || String(a.number).localeCompare(String(b.number), 'de', { numeric: true }))

/** `A-Dorf 07:00 → C-Stadt 07:42`: from its run where it has one, else its points and departure. */
export function trainLine(train, run) {
  const rows = run?.rows
  if (rows?.length) {
    const first = rows[0], last = rows[rows.length - 1]
    return `${first.name} ${formatClock(first.departure)} → ${last.name} ${formatClock(last.arrival)}`
  }
  const points = train.points ?? []
  if (!points.length) return `${formatClock(train.departure)}`
  return `${points[0].station} ${formatClock(train.departure)}${points.length > 1 ? ` → ${points[points.length - 1].station}` : ''}`
}

/** Where a stop at a station the train passes goes among its points: after those before it along the way. */
export function insertionIndex(run, offset) {
  const offsets = run.route.offsets
  let i = 0
  while (i < offsets.length && offsets[i] <= offset) i++
  return Math.max(1, Math.min(i, offsets.length - 1))
}

/**
 * The axis of the graphical timetable: the stations along a train's way,
 * in its running order, [{ key, name, at }] — `at` the distance from the
 * first [m].
 */
export function axisOf(rows) {
  const out = []
  for (const r of rows ?? []) {
    if (out.length && out[out.length - 1].key === r.key) continue
    out.push({ key: r.key, name: r.name, at: r.offset - rows[0].offset })
  }
  return out
}

/**
 * A train's lines over an axis, [[{ i, t }]] — `i` the station's place on the
 * axis, `t` the time —: its arrival and departure at each station of the axis
 * it runs through, a line broken where it leaves the axis.
 */
export function linesOn(axis, rows) {
  const index = new Map(axis.map((s, i) => [s.key, i]))
  const lines = []
  let line = null, before = null
  for (const r of rows ?? []) {
    const i = index.get(r.key)
    if (i == null) { line = null; continue }
    if (!line || Math.abs(i - before) !== 1) lines.push(line = [])
    if (r.arrival != null) line.push({ i, t: r.arrival })
    if (r.departure != null && r.departure !== r.arrival) line.push({ i, t: r.departure })
    before = i
  }
  return lines.filter(l => l.length >= 2)
}
