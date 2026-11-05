import { pathProperties } from './path.js'
import { signalledSpeeds } from './route.js'
import { rollingStockOf } from '@rail-across/vehicles'

/**
 * The request the simulation takes (rail-across-sim's SimulationRequest),
 * from a train's way over the net: the path's length, grades and speed
 * limits — the speeds the signals signal first, the layout's design speeds
 * only where no signal signals one (governedLimits) —,
 * the train — a vehicle, its traction `{ mode, comfort }` —, its stops and
 * the margin. Returns { request, properties }.
 *
 * `stops` [{ offset, duration, arrival }] along the path — the last at its
 * end, without a duration where the train runs on beyond it; `margin` {
 * value: '5%' | '4min/100km' | 'none', distribution: 'mareco' | 'linear' }.
 * `startSpeed` [m/s] the speed it has at the start — Infinity: what the line
 * and the train allow there —, `endSpeed` [m/s] the speed it leaves the
 * path's end at, at the most, or null; where it crosses a boundary of the
 * planning area.
 */
export function simulationRequest(record, path, { vehicle, traction = {}, stops = [], margin = null, startSpeed = 0, endSpeed = null, timeStep = 2 }) {
  const properties = pathProperties(record, path)
  const signalled = signalledSpeeds(record, path)
  const length = properties.length
  const limits = governedLimits(properties.speedLimits, signalled)
  if (endSpeed != null && Number.isFinite(endSpeed)) limits.push({ begin: Math.max(0, length - 1), end: length, speed: endSpeed, source: 'boundary' })
  const rollingStock = rollingStockOf(vehicle, traction)
  const schedule = stops.map(s => ({
    path_offset: Math.min(Math.max(s.offset, 0), length),
    ...(s.duration != null ? { stop_duration: s.duration } : {}),
    ...(s.arrival != null ? { arrival: s.arrival } : {}),
  }))
  const request = {
    path: {
      length,
      gradients: properties.gradients,
      speed_limits: limits,
    },
    rolling_stock: rollingStock,
    schedule,
    ...(margin?.value && margin.value !== 'none' ? { margins: { boundaries: [], values: [margin.value] } } : {}),
    distribution: margin?.distribution ?? 'mareco',
    initial_speed: startSpeed > 0 ? Math.min(startSpeed, startCap(limits, rollingStock)) : 0,
    time_step: timeStep,
  }
  // A design speed is missed only where no signal signals one either.
  return { request, properties: { ...properties, signalled, gaps: { ...properties.gaps, speed: uncovered(properties.gaps.speed, signalled) } } }
}

/**
 * The speed limits a train runs at: what the signals signal (`signalled`)
 * where they signal a speed — higher or lower than the design speed there —,
 * the design speeds (`design`) of the stretches no signal speaks for.
 */
export function governedLimits(design, signalled) {
  return [...uncovered(design, signalled), ...signalled].sort((a, b) => a.begin - b.begin)
}

/** The parts of spans along the path ([{ begin, end, … }]) that none of `covered` covers. */
function uncovered(spans, covered) {
  const by = [...covered].sort((a, b) => a.begin - b.begin)
  const out = []
  for (const d of spans) {
    let at = d.begin
    for (const c of by) {
      if (c.end <= at || c.begin >= d.end) continue
      if (c.begin > at) out.push({ ...d, begin: at, end: c.begin })
      at = Math.max(at, c.end)
      if (at >= d.end) break
    }
    if (at < d.end) out.push({ ...d, begin: at })
  }
  return out
}

/**
 * The fastest a train may run in at the start of a path: no faster than it
 * may run, and slow enough to come down to every limit ahead braking — a
 * little below its deceleration, as a gradient may take some of it.
 */
function startCap(limits, rollingStock) {
  const gamma = 0.8 * (rollingStock.const_gamma > 0 ? rollingStock.const_gamma : 0.5)
  let cap = rollingStock.max_speed
  for (const l of limits) cap = Math.min(cap, Math.sqrt(l.speed ** 2 + 2 * gamma * Math.max(0, l.begin)))
  return cap
}

/** A margin as the user writes it — 5, '5 %', '3 min/100km' — in the simulation's words, or null. */
export function marginValue(text) {
  const s = String(text ?? '').trim().replace(',', '.')
  if (!s) return 'none'
  const pct = s.match(/^(\d+(?:\.\d+)?)\s*%?$/)
  if (pct) return `${Number(pct[1])}%`
  const per = s.match(/^(\d+(?:\.\d+)?)\s*min\s*\/\s*100\s*km$/i)
  if (per) return `${Number(per[1])}min/100km`
  return null
}
