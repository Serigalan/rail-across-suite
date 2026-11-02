import { marginValue, routeVia, simulationRequest, stationNamed, stationTargets, stationsAlong, timeAtPosition } from '@rail-across/running-times'

/**
 * The timetables of the suite (decision 0016): a project's trains, each with
 * its number and kind, its vehicle of the catalogue, its departure and
 * margin, and its points — the station it starts at, the stations and halts
 * it stops at or passes, the station it ends at —, the stations named by
 * their names, so that one timetable is laid on any variant of its project.
 * A train begins and ends at a station (Bahnhof) only, on one of its tracks.
 *
 * train = { id, number, category, name, vehicle, mode, comfort, departure,
 *           margin, distribution, points }
 * point = { id, station, track, stop, dwell, departure }
 *
 * `departure` [s from midnight] at the first point; a point's `track` the
 * station's track it uses (an id of the variant's tracks), or null for any;
 * `stop` whether it stops there (it always does at its ends), `dwell` how
 * long [s], `departure` the time it waits for there before leaving, or null.
 */

/** The kinds of trains offered; any other is written as it is. */
export const CATEGORIES = ['ICE', 'IC', 'EC', 'RE', 'RB', 'S', 'IRE', 'Gz', 'Lz', 'Dz']

/** The colour of a kind of train in the graphical timetable. */
export function categoryColor(category) {
  const c = String(category ?? '').trim().toUpperCase()
  if (['ICE', 'IC', 'EC', 'ECE', 'NJ', 'D'].includes(c)) return '#c0392b'
  if (['RE', 'IRE'].includes(c)) return '#8e44ad'
  if (['RB', 'RS'].includes(c)) return '#303383'
  if (c === 'S') return '#1f8a4c'
  if (['GZ', 'GC', 'KLV', 'EK', 'DGS'].includes(c)) return '#8a5a2b'
  return '#5f6473'
}

const newId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`

export const newTimetable = () => ({ trains: [] })

export const newPoint = (station, patch = {}) => ({ id: newId(), station, track: null, stop: true, dwell: 60, departure: null, ...patch })

export const newTrain = (patch = {}) => ({
  id: newId(), number: '', category: 'RB', name: '', vehicle: null, mode: null, comfort: 'STANDARD',
  departure: 8 * 3600, margin: '5 %', distribution: 'mareco', points: [], ...patch,
})

/** A train copied: new ids, its departure and the departures it waits for moved by `shift` [s]. */
export function copyTrain(train, { shift = 0, number = train.number } = {}) {
  const moved = (t) => (t == null ? null : t + shift)
  return { ...structuredClone(train), id: newId(), number, departure: train.departure + shift, points: train.points.map(p => ({ ...p, id: newId(), departure: moved(p.departure) })) }
}

/**
 * A train's number moved on by `step` copies: `16001` by 2 is `16003`, the
 * digits at its end counted on — a number without digits stays as it is.
 */
export function numberAfter(number, step) {
  const m = String(number ?? '').match(/^(.*?)(\d+)$/)
  if (!m) return number
  const n = Number(m[2]) + step
  return n < 0 ? number : `${m[1]}${String(n).padStart(m[2].length, '0')}`
}

/**
 * Copies of a train at an interval (a Takt): `count` of them, each `every`
 * [s] after the one before, their numbers counted on by `numberStep`.
 */
export const taktCopies = (train, { every, count, numberStep = 2 }) =>
  Array.from({ length: count }, (_, i) => copyTrain(train, { shift: every * (i + 1), number: numberAfter(train.number, numberStep * (i + 1)) }))

/** A clock time as one writes it — 7:05, 07:05:30, 0705 — in seconds from midnight, or null. Past 24:00 is the next day. */
export function parseClock(text) {
  const s = String(text ?? '').trim()
  let m = s.match(/^(\d{1,2})[:.](\d{2})(?:[:.](\d{2}))?$/) ?? s.match(/^(\d{2})(\d{2})(\d{2})?$/)
  if (!m) return null
  const [h, min, sec] = [Number(m[1]), Number(m[2]), Number(m[3] ?? 0)]
  if (h > 47 || min > 59 || sec > 59) return null
  return h * 3600 + min * 60 + sec
}

/** Seconds from midnight as a clock time, 07:05 or with `seconds` 07:05:30; the next day's from 24:00 on, as they are. */
export function formatClock(seconds, { seconds: withSeconds = false } = {}) {
  if (!Number.isFinite(seconds)) return '–'
  const s = Math.round(seconds)
  const pad = (n) => String(n).padStart(2, '0')
  const hm = `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}`
  return withSeconds ? `${hm}:${pad(s % 60)}` : hm
}

/** A train as one line: `RB 16001`, its name after it where it has one. */
export const trainTitle = (train) => [[train.category, train.number].filter(Boolean).join(' '), train.name].filter(Boolean).join(' · ')

/**
 * What keeps a train from being run, as [{ key, point? }] — `point` the index
 * of the point it is about: too few points, a first or last point that is no
 * station of the net (a train begins and ends at a station only), a station or
 * track the variant does not have, no vehicle, a margin that cannot be read.
 */
export function trainProblems(train, stations, { vehicle = true } = {}) {
  const out = []
  const points = train.points ?? []
  if (points.length < 2) out.push({ key: 'too_few_points' })
  points.forEach((p, i) => {
    const station = stationNamed(stations, p.station)
    const end = i === 0 || i === points.length - 1
    if (!station) out.push({ key: 'station_missing', point: i })
    else if (end && points.length >= 2 && station.kind !== 'station') out.push({ key: i === 0 ? 'start_not_station' : 'end_not_station', point: i })
    else if (p.track && !station.tracks.some(tr => tr.id === p.track) && !station.platforms.some(pl => pl.trackId === p.track)) out.push({ key: 'track_missing', point: i })
  })
  if (!vehicle) out.push({ key: 'no_vehicle' })
  if (marginValue(train.margin) == null) out.push({ key: 'margin_invalid' })
  return out
}

/** Where each point of a train may be on the net: its station's tracks (stationTargets), the ends stopping always. */
export function routePoints(train, stations) {
  const last = train.points.length - 1
  return train.points.map((p, i) => stationTargets(stationNamed(stations, p.station), { trackId: p.track, stop: p.stop || i === 0 || i === last }))
}

/**
 * The way of a train over the net and the request for its run: { route,
 * request, properties, stops } — `stops` the indices of the points it stops
 * at after its start, as the request's schedule has them —, or { failedAt }
 * the point it found no way to.
 */
export function trainRequest(record, stations, train, vehicle) {
  const route = routeVia(record, routePoints(train, stations))
  if (!route.path) return { failedAt: route.failedAt }
  const last = train.points.length - 1
  const stops = train.points.map((p, i) => i).filter(i => i > 0 && (i === last || train.points[i].stop))
  const { request, properties } = simulationRequest(record, route.path, {
    vehicle, traction: { mode: train.mode, comfort: train.comfort ?? 'STANDARD' },
    stops: stops.map(i => ({ offset: route.offsets[i], duration: i === last ? 0 : train.points[i].dwell ?? 0 })),
    margin: { value: marginValue(train.margin), distribution: train.distribution ?? 'mareco' },
  })
  return { route, request, properties, stops }
}

/**
 * The times of a train's run, as the timetable shows them: one row for each
 * station along its way — those it stops at, those it passes —, in running
 * order, { key, name, offset, point, stop, arrival, departure, track } —
 * `point` the index of the train's point there, or null where it only passes;
 * a pass's arrival its departure. Where it waits for a departure, the run on
 * from there is that much later. Times in seconds from midnight.
 */
export function trainTimes(train, stations, { route, stops }, result) {
  const final = result.final
  const at0 = train.departure
  // the simulation's times at the stops, and how much later each is left for a departure waited for
  const sched = new Map(stops.map((i, k) => [i, final.scheduled_points[k]]))
  const shifts = []   // [{ offset, shift }] from that offset on
  let shift = 0
  for (const i of stops) {
    const s = sched.get(i)
    const wait = train.points[i].departure
    if (wait != null && i < train.points.length - 1) {
      const leaves = at0 + s.departure + shift
      if (wait > leaves) shift += wait - leaves
    }
    shifts.push({ offset: route.offsets[i], shift })
  }
  const shiftBefore = (offset) => { let v = 0; for (const s of shifts) if (s.offset < offset - 1e-6) v = s.shift; return v }
  const shiftAt = (offset) => { let v = 0; for (const s of shifts) if (s.offset <= offset + 1e-6) v = s.shift; return v }

  const along = stationsAlong(route.path, stations)
  const rows = []
  const used = new Set()
  const last = train.points.length - 1
  for (const s of along) {
    const i = train.points.findIndex((p, j) => !used.has(j) && stationNamed(stations, p.station)?.key === s.key
      && route.offsets[j] >= s.begin - 1e-6 && route.offsets[j] <= s.end + 1e-6)
    if (i >= 0) used.add(i)
    rows.push({ key: s.key, name: s.name, offset: i >= 0 ? route.offsets[i] : s.at, point: i >= 0 ? i : null, track: i >= 0 ? route.chosen[i].trackId : null })
  }
  // points the stations along did not find — a station whose tracks the way only touches at their end
  train.points.forEach((p, i) => {
    if (used.has(i)) return
    const station = stationNamed(stations, p.station)
    rows.push({ key: station?.key ?? p.station, name: station?.name ?? p.station, offset: route.offsets[i], point: i, track: route.chosen[i].trackId })
  })
  rows.sort((a, b) => a.offset - b.offset || (a.point ?? 0) - (b.point ?? 0))
  return rows.map(r => {
    const stopping = r.point != null && (r.point === 0 || r.point === last || train.points[r.point].stop)
    let arrival, departure
    if (r.point === 0) { arrival = null; departure = at0 }
    else if (stopping) {
      const s = sched.get(r.point)
      arrival = at0 + s.arrival + shiftBefore(r.offset)
      departure = r.point === last ? null : at0 + s.departure + shiftAt(r.offset)
    } else {
      const t = at0 + (timeAtPosition(final, r.offset) ?? 0) + shiftBefore(r.offset)
      arrival = t
      departure = t
    }
    return { ...r, stop: stopping, arrival, departure }
  })
}

/**
 * A train run: its way and request, simulated with `simulate` (the module's,
 * async), and its times. { route, request, properties, result, rows } — or {
 * problems }, { failedAt } or { error } where it cannot be run.
 */
export async function runTrain(record, stations, train, vehicle, simulate) {
  const problems = trainProblems(train, stations, { vehicle: Boolean(vehicle) })
  if (problems.length) return { problems }
  const made = trainRequest(record, stations, train, vehicle)
  if (made.failedAt != null) return { failedAt: made.failedAt }
  const result = await simulate(made.request)
  if (result.error) return { error: result.error }
  return { ...made, result, rows: trainTimes(train, stations, made, result) }
}

/** The key a train's run is known by: what it is run with, so a run is only made again when that changes. */
export const runKey = (train, entry) => JSON.stringify([train.points, train.departure, train.mode, train.comfort, train.margin, train.distribution, entry?.key, entry?.version])
