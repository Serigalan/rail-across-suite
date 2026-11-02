import { SIGNAL_CATALOGUE } from './catalogue.js'
import { heightAt } from '../track/index.js'

// What an overlap needs: the speed a route leads up to its target with — the
// speed its start signal signals for it — and the length that speed and the
// governing gradient ask for, after the catalogue's table (durchrutschweg,
// a draft of Ril 819.0202).

/**
 * The speed a route permits [km/h]: what a logical signal at its start signals
 * for it (signallingSpeeds), or null where none does.
 */
export function signallingSpeedOf(signalling, route) {
  const start = (signalling?.collections?.signals ?? []).find(s => s.id === route?.startSignalId)
  for (const l of start?.logicalSignals ?? []) {
    const hit = l.signallingSpeeds?.find(x => x.routeId === route.id)
    if (hit) return hit.speed
  }
  return null
}

/**
 * The length an overlap needs [m] for a speed [km/h] and the governing
 * gradient over it [‰, negative falling in the way it runs]: the length of the
 * speed's step, longer on a fall steeper than the table's limit. Null without
 * a speed.
 */
export function overlapLength(speed, gradient = 0, catalogue = SIGNAL_CATALOGUE) {
  if (speed == null || !Number.isFinite(speed)) return null
  const table = catalogue.durchrutschweg
  const base = (table.stufen.find(s => s.bis == null || speed <= s.bis) ?? table.stufen.at(-1)).laenge
  const { ab_gefaelle: from, verzoegerung: a, schwere: g, runden: step } = table.neigung
  const fall = -(gradient ?? 0)
  if (fall <= from) return base
  const left = a - (g * fall) / 1000
  if (left <= 0) return null
  return Math.ceil((base * a) / left / step) * step
}

/**
 * The gradient over a run from `start` to `end` [‰], rising positive in the
 * way it runs: the heights of the tracks at both ends over the length run
 * between them. Null where a track has no heights or the length is none.
 */
export function gradientAlong(tracks, start, end, length) {
  if (!(length > 0)) return null
  const heightOf = (loc) => heightAt(tracks.find(t => t.id === loc?.trackId)?.heights, loc?.station)
  const a = heightOf(start), b = heightOf(end)
  if (a == null || b == null) return null
  return Math.round(((b - a) / length) * 1000 * 10) / 10
}
