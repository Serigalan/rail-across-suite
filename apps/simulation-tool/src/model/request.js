import { pathProperties } from './path.js'
import { signalledSpeeds } from './route.js'
import { rollingStockOf } from './vehicles.js'

/**
 * The request the simulation takes (rail-across-sim's SimulationRequest),
 * from a train's way over the net: the path's length, grades and speed
 * limits — the layout's design speeds and the speeds the signals signal —,
 * the train, its stops and the margin. Returns { request, properties }.
 *
 * `stops` [{ offset, duration, arrival }] along the path — the last at its
 * end; `margin` { value: '5%' | '4min/100km' | 'none', distribution:
 * 'mareco' | 'linear' }.
 */
export function simulationRequest(record, path, { vehicle, stops = [], margin = null, timeStep = 2 }) {
  const properties = pathProperties(record, path)
  const signalled = signalledSpeeds(record, path)
  const length = properties.length
  const schedule = stops.map(s => ({
    path_offset: Math.min(Math.max(s.offset, 0), length),
    ...(s.duration != null ? { stop_duration: s.duration } : {}),
    ...(s.arrival != null ? { arrival: s.arrival } : {}),
  }))
  const request = {
    path: {
      length,
      gradients: properties.gradients,
      speed_limits: [...properties.speedLimits, ...signalled],
    },
    rolling_stock: rollingStockOf(vehicle),
    schedule,
    ...(margin?.value && margin.value !== 'none' ? { margins: { boundaries: [], values: [margin.value] } } : {}),
    distribution: margin?.distribution ?? 'mareco',
    initial_speed: 0,
    time_step: timeStep,
  }
  return { request, properties: { ...properties, signalled } }
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
