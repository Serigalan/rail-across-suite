import { aspectOf, aspectSpeed, branchSpeedOnRun } from '@rail-across/core/signalling'
import { replaced, signalsSpeed, withRouteSpeed } from './domain.js'
import { actsAlong, nextSignals, routeStretches, runStretches } from './routeSearch.js'

/**
 * The speeds the elements give: a route is run at the lowest design speed of
 * the track elements it runs over and of the turnouts it passes on their
 * branch, up to the first speed signal (Lf 7) on it that acts its way; each
 * Lf 7 shows the lowest from where it stands to the next speed signal — a
 * Zs 3 or an Lf 7 — ahead, over every way the net offers there: the board is
 * the same for every route past it. Zs 3 and Lf 7 show tens: a speed is rounded down,
 * and a route's start signal gets the Zs 3 that shows it (withRouteSpeed).
 */

/** The aspect of the speed signal. */
export const SPEED_SIGNAL = 'Lf_7'
const LF7 = aspectOf(SPEED_SIGNAL)
const SYMBOLS = LF7.symbole.map(Number)

/** The Lf 7 a signal shows on a plate of its own, or null. */
export const speedAspectOf = (s) => (s?.frames ?? []).flatMap(f => f.aspects ?? []).find(a => a.aspect === SPEED_SIGNAL) ?? null

/** Whether a signal is a speed signal on a track: a post with a plate Lf 7. */
export const isSpeedSignal = (s) => s?.kind === 'real' && s.at?.anchor === 'station' && !!speedAspectOf(s)

/** The speed an Lf 7 shows [km/h], or null. */
export const speedSignalSpeed = (s) => aspectSpeed(LF7, speedAspectOf(s)?.symbol)

/** The symbol an Lf 7 shows for a speed: its tens, rounded down, within those the board has; null for none. */
export function speedSymbol(speed) {
  if (!(speed > 0)) return null
  const tens = Math.floor(speed / 10)
  return String(Math.max(SYMBOLS[0], Math.min(SYMBOLS[SYMBOLS.length - 1], tens)))
}

/** A speed signal showing `speed`: every Lf 7 on it with that symbol, none where it is null. */
export function withSpeedSignalSpeed(signal, speed) {
  const symbol = speedSymbol(speed)
  return {
    ...signal,
    frames: signal.frames.map(f => ({
      ...f,
      aspects: f.aspects.map(a => {
        if (a.aspect !== SPEED_SIGNAL) return a
        const { symbol: _old, ...rest } = a
        return symbol ? { ...rest, symbol } : rest
      }),
    })),
  }
}

/**
 * The lowest speed the elements allow over stretches of track ([{ trackId, lo,
 * hi, direction }], in running order): { speed, where } — `where` the track
 * element ({ kind: 'track', trackId, from, to }) or the turnout ({ kind:
 * 'switch', switchId }) that sets it —, or null where none says a speed.
 */
export function elementSpeed(net, stretches) {
  let best = null
  const take = (speed, where) => { if (speed > 0 && (!best || speed < best.speed)) best = { speed, where } }
  const byId = new Map(net.tracks.map(t => [t.id, t]))
  for (const st of stretches) {
    let s = 0
    for (const el of byId.get(st.trackId)?.elements ?? []) {
      const len = el.length ?? 0
      const lo = Math.max(s, st.lo), hi = Math.min(s + len, st.hi)
      // An element counts where the run covers some of it, and a run shorter than half a metre counts the one it stands on.
      if (hi - lo > 0.5 || (len > 0 && hi >= lo && st.hi - st.lo <= 0.5)) take(el.speed, { kind: 'track', trackId: st.trackId, from: lo, to: hi })
      s += len
    }
  }
  for (let i = 0; i + 1 < stretches.length; i++) {
    const pair = [stretches[i].trackId, stretches[i + 1].trackId]
    if (pair[0] === pair[1]) continue
    const branch = branchSpeedOnRun(net.switches, pair)
    if (branch?.speed) take(branch.speed, { kind: 'switch', switchId: branch.switchIds[0] })
  }
  return best
}

/** The part of stretches between two distances along them [m]. */
export function cut(stretches, from, to) {
  const out = []
  let at = 0
  for (const st of stretches) {
    const len = st.hi - st.lo
    const a = Math.max(from, at), b = Math.min(to, at + len)
    if (b > a || (b === a && from === to && a >= at && a <= at + len)) {
      out.push(st.direction === 'down'
        ? { ...st, lo: st.hi - (b - at), hi: st.hi - (a - at) }
        : { ...st, lo: st.lo + (a - at), hi: st.lo + (b - at) })
    }
    at += len
  }
  return out
}

/**
 * The Lf 7 standing on stretches of track and acting their way, and how far
 * along they stand [m]: [{ signal, at }], in running order. One at the very
 * start or end does not count — it stands before or past the run.
 */
export function speedSignalsOn(stretches, signals) {
  const boards = signals.filter(isSpeedSignal)
  const out = []
  let at = 0
  for (const st of stretches) {
    for (const s of boards) {
      if (s.at.trackId !== st.trackId || !actsAlong(s, st.direction)) continue
      if (s.at.station <= st.lo + 0.5 || s.at.station >= st.hi - 0.5) continue
      out.push({ signal: s, at: at + (st.direction === 'down' ? st.hi - s.at.station : s.at.station - st.lo) })
    }
    at += st.hi - st.lo
  }
  return out.sort((a, b) => a.at - b.at)
}

/**
 * A route's speed sections, cut at the Lf 7 on it: [{ signal, from, to,
 * elements }] — the first from its start signal, the others each from an
 * Lf 7; `elements` what the elements of the section allow (elementSpeed).
 */
export function routeSpeedSections(net, route, signals) {
  const stretches = routeStretches(net, route, signals)
  const length = stretches.reduce((sum, st) => sum + st.hi - st.lo, 0)
  const start = signals.find(s => s.id === route.startSignalId)
  const marks = [{ signal: start, at: 0 }, ...speedSignalsOn(stretches, signals)]
  return marks.map((m, i) => {
    const to = marks[i + 1]?.at ?? length
    return { signal: m.signal, from: m.at, to, elements: elementSpeed(net, cut(stretches, m.at, to)) }
  })
}

/** What a route is run at as the elements give it: the lowest speed up to its first Lf 7 — { speed, where } or null. */
export const routeElementSpeed = (net, route, signals) => routeSpeedSections(net, route, signals)[0]?.elements ?? null

/** Whether a signal shows a speed: an Lf 7, or a Zs 3 — on a plate of its own or among its logical signals' indicators. */
export const showsSpeed = (s) => s?.kind === 'real' && s.at?.anchor === 'station' && (isSpeedSignal(s)
  || [...(s.frames ?? []), ...(s.logicalSignals ?? []).flatMap(l => l.frames ?? [])].some(f => (f.aspects ?? []).some(a => a.aspect === 'Zs_3')))

/**
 * How far an Lf 7's speed holds: over every way the net offers ahead of it
 * to the next speed signal — a Zs 3 or an Lf 7 —, to where the net ends
 * where there is none. { ways: [{ target, trackIds, length, stretches,
 * elements }], elements } — each way with what its elements allow
 * (elementSpeed), and the lowest of them all, which the Lf 7 shows.
 */
export function speedSignalReach(net, signals, signal) {
  const ways = nextSignals(net, signals, signal, { isTarget: showsSpeed, ends: true }).map(way => {
    const stretches = runStretches(net, signal.at, way.trackIds, way.target?.at)
    return { ...way, stretches, elements: elementSpeed(net, stretches) }
  })
  const elements = ways.map(w => w.elements).filter(Boolean).sort((a, b) => a.speed - b.speed)[0] ?? null
  return { ways, elements }
}

/** What an Lf 7 at a place shows as the elements give it (speedSignalReach): { speed, where } or null. */
export const speedSignalElementSpeed = (net, signals, signal) => speedSignalReach(net, signals, signal).elements

/** speedSignalElementSpeed's speed for an Lf 7 anywhere on the net, worked out once a metre. */
export function speedSignalSpeeds(net, signals) {
  const known = new Map()
  return (s) => {
    const key = `${s.id}|${s.at.trackId}|${Math.round(s.at.station)}|${s.at.direction}`
    if (!known.has(key)) known.set(key, speedSignalElementSpeed(net, signals, s)?.speed ?? null)
    return known.get(key)
  }
}

/**
 * The domain with the speeds the elements give set: every train route (of
 * `routeIds`, or all) signals what its first section allows, every Lf 7
 * shows what lies ahead of it (unless `boards` is false). { domain, routes,
 * boards } — how many routes and Lf 7 it changed.
 */
export function withElementSpeeds(domain, net, { routeIds = null, boards: withBoards = true } = {}) {
  let next = domain
  let boards = 0, routes = 0
  for (const s of withBoards ? domain.collections.signals.filter(isSpeedSignal) : []) {
    const got = speedSignalElementSpeed(net, domain.collections.signals, s)
    if (!got || speedSymbol(got.speed) === speedAspectOf(s).symbol) continue
    next = replaced(next, 'signals', withSpeedSignalSpeed(s, got.speed))
    boards++
  }
  for (const route of domain.collections.routes) {
    if (route.kind !== 'train' || (routeIds && !routeIds.includes(route.id))) continue
    const got = routeElementSpeed(net, route, next.collections.signals)
    if (!got || signalsSpeed(next, route, got.speed)) continue
    const set = withRouteSpeed(next, route, got.speed)
    if (set === next) continue
    next = set
    routes++
  }
  return { domain: next, routes, boards }
}
