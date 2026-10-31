import { gradientAlong, overlapLength, signallingSpeedOf } from '@rail-across/core/signalling'
import { newId, replaced } from './domain.js'
import { runAhead } from './runs.js'

/**
 * An overlap measured for a speed: the length the speed asks (catalogue:
 * durchrutschweg, a draft of Ril 819.0202), run ahead of its signal — straight
 * on at every switch — and the gradient over that run taken from the heights
 * of the tracks, the length asked again for that gradient, until it holds:
 * { path, end, length, gradient, speed }. Null without a speed or a place.
 */
export function measuredOverlap(net, signal, speed) {
  if (signal?.at?.anchor !== 'station') return null
  let length = overlapLength(speed, 0)
  if (length == null) return null
  let run = null, gradient = null
  for (let i = 0; i < 4; i++) {
    run = runAhead(net, signal.at, length)
    if (!run) return null
    gradient = gradientAlong(net.tracks, signal.at, run.end, length)
    const asked = overlapLength(speed, gradient ?? 0)
    if (asked == null || asked === length) break
    length = asked
  }
  return {
    path: { trackIds: run.trackIds }, end: { anchor: 'station', ...run.end }, length, speed,
    ...(gradient != null ? { gradient } : {}),
  }
}

/**
 * A train route's overlap at its target, measured from the speed its start
 * signal signals for it: the one it prefers (or its only one) measured anew,
 * else a new one it names, preferred. { domain } with both, or { problem }:
 * 'speed' where no speed is signalled for it, 'target' where its target stands
 * on no track.
 */
export function withMeasuredOverlap(domain, net, route) {
  const speed = signallingSpeedOf(domain, route)
  if (speed == null) return { problem: 'speed' }
  const target = domain.collections.signals.find(s => s.id === route.targetSignalId)
  const measured = measuredOverlap(net, target, speed)
  if (!measured) return { problem: 'target' }
  const named = route.overlaps ?? []
  const keepId = (named.find(o => o.preferred) ?? (named.length === 1 ? named[0] : null))?.overlapId
  const existing = keepId && domain.collections.overlaps.find(o => o.id === keepId)
  if (existing) {
    // A gradient measured before goes where there are no heights to measure one now.
    const { gradient: _before, ...rest } = existing
    return { domain: replaced(domain, 'overlaps', { ...rest, ...measured }) }
  }
  const atTarget = domain.collections.overlaps.filter(o => o.signalId === target.id).length
  const overlap = { id: newId(), name: `D${atTarget + 1}`, signalId: target.id, ...measured }
  const withOverlap = { ...domain, collections: { ...domain.collections, overlaps: [...domain.collections.overlaps, overlap] } }
  return { domain: replaced(withOverlap, 'routes', { ...route, overlaps: [...named, { overlapId: overlap.id, ...(named.some(o => o.preferred) ? {} : { preferred: true }) }] }) }
}

/** The speed an overlap is measured for: the one it allows, else the highest a train route leading to it signals. */
export function overlapSpeed(domain, overlap) {
  if (overlap.speed != null) return overlap.speed
  const speeds = domain.collections.routes
    .filter(r => r.kind === 'train' && (r.overlaps ?? []).some(o => o.overlapId === overlap.id))
    .map(r => signallingSpeedOf(domain, r)).filter(v => v != null)
  return speeds.length ? Math.max(...speeds) : null
}
