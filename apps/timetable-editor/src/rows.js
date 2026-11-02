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

/**
 * A train's blocking times over an axis (its blocks, occupancy.js), [{ a0,
 * a1, begin, end, block }] — `a0`, `a1` the distances along the axis the
 * block spans [m] —: each block's stretch of the way mapped between the
 * stations of the axis it runs through, as its line is drawn.
 */
export function blocksOn(axis, rows, blocks) {
  const index = new Map(axis.map((s, i) => [s.key, i]))
  // the train's offsets at the axis's stations, in running order, and where it leaves the axis
  const marks = []
  for (const r of rows ?? []) {
    const i = index.get(r.key)
    if (i == null) { marks.push(null); continue }
    if (marks.at(-1)?.i !== i) marks.push({ i, offset: r.offset })
  }
  const out = []
  for (const b of blocks ?? []) {
    let a0 = null, a1 = null
    for (let k = 1; k < marks.length; k++) {
      const p = marks[k - 1], q = marks[k]
      if (!p || !q || Math.abs(p.i - q.i) !== 1 || q.offset <= p.offset) continue
      const lo = Math.max(b.from, p.offset), hi = Math.min(b.reach, q.offset)
      if (hi < lo) continue
      const at = (o) => axis[p.i].at + (o - p.offset) / (q.offset - p.offset) * (axis[q.i].at - axis[p.i].at)
      for (const v of [at(lo), at(hi)]) { a0 = a0 == null ? v : Math.min(a0, v); a1 = a1 == null ? v : Math.max(a1, v) }
    }
    if (a0 != null) out.push({ a0, a1, begin: b.begin, end: b.end, block: b.index })
  }
  return out
}
