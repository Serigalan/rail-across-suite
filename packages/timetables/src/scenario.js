import { pathLength } from '@rail-across/running-times'
import { DISPATCH, occupancy } from './occupancy.js'

/**
 * A timetable run in a construction scenario (decision 0014): on the net of a
 * day, with the scenario's stretches of track closed while they hold, its
 * trains taken out left out, its trains diverted by hand on their points. A
 * train the closures touch is diverted the shortest way around them; one with
 * no way left is not runnable. The trains are run through the dispatching as
 * the Fahrplan-Editor runs them, and set beside their regular runs.
 *
 * A scenario: { id, name, closures: [{ id, name, at: { trackId, startStation,
 * endStation }, firstDay, lastDay, window: { from, to } }], cancelledTrainIds,
 * diversions: [{ trainId, points }] } — days as `YYYY-MM-DD`, a window in
 * seconds from midnight, `to` before `from` past midnight.
 */

const DAY = 86400

/** A day `k` days on from `day` (`YYYY-MM-DD`). */
export function dayAfter(day, k = 1) {
  const [y, m, d] = String(day).split('-').map(Number)
  return new Date(Date.UTC(y, m - 1, d + k)).toISOString().slice(0, 10)
}

/** The stretch a closure closes: { trackId, lo, hi }. */
const stretchOf = (c) => ({ trackId: c.at.trackId, lo: Math.min(c.at.startStation, c.at.endStation), hi: Math.max(c.at.startStation, c.at.endStation) })

/**
 * The times a closure holds around a day, seconds from that day's midnight:
 * [[from, to]] — the day before, the day and the day after, each where it lies
 * within the closure's days; all day, or its window (past midnight into the
 * next day where `to` is before `from`).
 */
export function closureTimes(closure, day) {
  const out = []
  for (const k of [-1, 0, 1]) {
    const d = dayAfter(day, k)
    if (d < closure.firstDay || d > closure.lastDay) continue
    const w = closure.window
    if (!w) out.push([k * DAY, (k + 1) * DAY])
    else out.push([k * DAY + w.from, k * DAY + (w.to > w.from ? w.to : w.to + DAY)])
  }
  return out
}

/** The closures of a scenario that hold at some time of a day, or reach into it from the day before. */
export const closuresAround = (scenario, day) => (scenario?.closures ?? []).filter(c => closureTimes(c, day).some(([a, b]) => b > 0 && a < DAY))

/** A record with what `closures` close: the way runs over none of it (findPath's `closed`). */
export const closedRecord = (record, closures) => (closures.length ? { ...record, closed: closures.map(stretchOf) } : record)

/**
 * When a run is on a stretch of track: [[t0, t1]] for each time its way runs
 * over it — from its head entering to its tail leaving —, seconds from
 * midnight; none where it does not run over it.
 */
export function timesOn(run, stretch, trainLength) {
  const path = run?.route?.path
  if (!path || !run.clock) return []
  const length = pathLength(path)
  const out = []
  let at = 0
  for (const p of path) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const a = Math.max(lo, stretch.lo), b = Math.min(hi, stretch.hi)
    if (p.trackId === stretch.trackId && b - a > 0.5) {
      const [o1, o2] = [at + Math.abs(a - p.start), at + Math.abs(b - p.start)].sort((x, y) => x - y)
      out.push([run.clock.first(o1), run.clock.last(Math.min(o2 + trainLength, length))])
    }
    at += hi - lo
  }
  return out
}

/** The closures a run passes while they hold on a day — its head on the stretch to its tail off it, within a time it holds. */
export function touchedBy(run, closures, day, trainLength) {
  return closures.filter(c => {
    const on = timesOn(run, stretchOf(c), trainLength)
    return on.length && closureTimes(c, day).some(([a, b]) => on.some(([t0, t1]) => Math.min(b, t1) > Math.max(a, t0)))
  })
}

/** How a train fares in a scenario. */
export const SCENARIO_STATES = ['regular', 'diverted', 'by_hand', 'not_runnable', 'cancelled']

/**
 * The runs of a timetable's trains in a scenario on a day: Map id → { state,
 * closures, train, run } — `state` one of SCENARIO_STATES, `closures` the ids
 * of those it is kept off, `train` as it runs (a diversion's points), `run`
 * as occupancy gives it (with its blocks, waits, conflicts), or the run that
 * found no way (`failedAt`) —, a train taken out with neither.
 *
 * `run(record, train, holds, routes)` runs a train (async; runTrain with its
 * vehicle and the simulation); `regular` the trains' runs in the timetable
 * as it is, on the net of the day — where a train lacks one it is run alone;
 * `dispatch` how conflicts are dealt with.
 */
export async function scenarioRuns(record, trains, scenario, day, { run, regular = new Map(), dispatch = DISPATCH }) {
  const cancelled = new Set(scenario?.cancelledTrainIds ?? [])
  const byHand = new Map((scenario?.diversions ?? []).map(d => [d.trainId, d]))
  const closures = closuresAround(scenario, day)
  const out = new Map()
  const plans = []
  for (const train of trains) {
    if (cancelled.has(train.id)) { out.set(train.id, { state: 'cancelled', closures: [], train }); continue }
    const diversion = byHand.get(train.id)
    const own = diversion ? { ...train, points: diversion.points } : train
    const plain = regular.get(train.id)?.rows ? regular.get(train.id) : await run(record, train, [], [])
    const length = (r) => r?.request?.rolling_stock?.length ?? 0
    let closed = plain?.rows ? touchedBy(plain, closures, day, length(plain)) : []
    if (!diversion && !closed.length) { plans.push({ train, own, closed: [], state: 'regular' }); continue }
    // Kept off what it passes while closed, and again off what the way around passes, a few times at the most.
    let tried = null
    for (let i = 0; i < 3; i++) {
      tried = await run(closedRecord(record, closed), own, [], [])
      if (!tried?.rows) break
      const more = touchedBy(tried, closures, day, length(tried)).filter(c => !closed.includes(c))
      if (!more.length) break
      closed = [...closed, ...more]
    }
    if (!tried?.rows) { out.set(train.id, { state: 'not_runnable', closures: closed.map(c => c.id), train: own, run: tried }); continue }
    plans.push({ train, own, closed, state: diversion ? 'by_hand' : 'diverted' })
  }
  const recordOf = new Map(plans.map(p => [p.train.id, closedRecord(record, p.closed)]))
  const placed = await occupancy(record, plans.map(p => p.own), (t, holds, routes) => run(recordOf.get(t.id) ?? record, t, holds, routes), dispatch)
  for (const p of plans) out.set(p.train.id, { state: p.state, closures: p.closed.map(c => c.id), train: p.own, run: placed.get(p.train.id) })
  return new Map(trains.filter(t => out.has(t.id)).map(t => [t.id, out.get(t.id)]))
}

/**
 * A scenario's trains beside their regular runs, in the timetable's order:
 * [{ id, state, closures, departure, arrival, regularArrival, runningTime,
 * regularRunningTime, delay, conflicts, waits, failedAt }] — times in seconds
 * from midnight, `delay` how much later it arrives than as it is timetabled
 * (its regular run), null where it does not run.
 */
export function scenarioRows(trains, regular, runs) {
  return trains.map(train => {
    const s = runs.get(train.id)
    const r = regular.get(train.id)
    const clock = s?.run?.clock, before = r?.clock
    return {
      id: train.id, state: s?.state ?? 'regular', closures: s?.closures ?? [],
      departure: clock?.at0 ?? null, arrival: clock?.end ?? null,
      regularArrival: before?.end ?? null,
      runningTime: clock ? clock.end - clock.at0 : null,
      regularRunningTime: before ? before.end - before.at0 : null,
      delay: clock && before ? clock.end - before.end : null,
      conflicts: s?.run?.conflicts?.length ?? 0, waits: s?.run?.waits?.length ?? 0,
      failedAt: s?.run?.failedAt ?? null, problems: s?.run?.problems ?? null,
    }
  })
}
