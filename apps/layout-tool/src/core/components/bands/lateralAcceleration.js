/**
 * The lateral acceleration a reference vehicle feels along a track (the
 * Seitenbeschleunigung band of the Trassenbänder), at its centre of gravity
 * REFERENCE_VEHICLE.height above the top of rail, in the plane of the track:
 *
 *   a_q = v² · κ · cos φ − g · sin φ + h · φ̈        φ = asin(u / s)
 *
 * — the speed v, the curvature κ and the cant u of the track (s the distance
 * of the running circles, 1.5 m), signed as the curvature is: positive
 * outwards of a right curve. The last term is the roll of the vehicle in a
 * cant ramp: its body turns about the rail plane, and its centre of gravity,
 * h above it, is swung sideways with it. A ramp is linear, so its roll rate
 * jumps at either end; φ̈ is taken as the change of the roll rate over the
 * length of the vehicle (REFERENCE_VEHICLE.base, about the bogie centres of
 * a coach) — φ̈ = v² · (φ′(x + L/2) − φ′(x − L/2)) / L, the second
 * derivative averaged over L —, so either end of a straight ramp is a step up
 * and down L long rather than a spike. The curved ramp of a Bloss transition
 * (3t² − 2t³) starts and ends flat: its roll rate is read from its own form,
 * and the band runs smooth over it. The speed is taken as constant along each
 * element, as the design speed states it.
 */

/** The reference vehicle: its centre of gravity over the top of rail [m], the length its roll is averaged over [m]. */
export const REFERENCE_VEHICLE = { height: 1.7, base: 18 }

/** The distance of the running circles [m] and gravity [m/s²], as physics.json states them. */
const RUNNING_CIRCLES = 1.5
const G = 9.81

/** A band's value at x, between its points; null outside it or where a value is missing. */
function valueAt(band, x) {
  for (let i = 1; i < band.length; i++) {
    const a = band[i - 1], b = band[i]
    if (x < a.s || x > b.s || b.s <= a.s) continue
    if (a.v == null || b.v == null) return null
    return a.v + (b.v - a.v) * (x - a.s) / (b.s - a.s)
  }
  return null
}

/** The roll angle φ [rad] of a cant u [mm]. */
const rollOf = (u) => Math.asin(Math.max(-1, Math.min(1, u / 1000 / RUNNING_CIRCLES)))

/** Whether an element carries the curved ramp: a Bloss transition's. */
const curvedRamp = (el) => el?.elementType === 'transition' && el.transitionType === 'bloss'

/**
 * The ramps of a track: each element's span with the cant and the curvature
 * at its two ends, from the bands at the span's ends, and whether it is a
 * Bloss transition, where both change along 3t² − 2t³.
 */
function rampsOf(spans, cant, curvature) {
  return spans.map(sp => {
    const e = Math.min(1e-6, (sp.to - sp.from) / 4)
    return {
      from: sp.from, to: sp.to, curved: curvedRamp(sp.el),
      u0: valueAt(cant, sp.from + e) ?? 0, u1: valueAt(cant, sp.to - e) ?? 0,
      k0: valueAt(curvature, sp.from + e), k1: valueAt(curvature, sp.to - e),
    }
  })
}

/** The curvature [1/km] at x: straight between the ends, along 3t² − 2t³ on a Bloss transition. */
function curvatureAt(ramps, curvature, x) {
  const r = rampAt(ramps, x)
  if (!r || !r.curved || r.k0 == null || r.k1 == null) return valueAt(curvature, x)
  const t = (x - r.from) / (r.to - r.from)
  return r.k0 + (r.k1 - r.k0) * t * t * (3 - 2 * t)
}

/** The ramp at x: the span holding it, else null beyond the track. */
const rampAt = (ramps, x) => ramps.find(r => x >= r.from && x <= r.to && r.to > r.from) ?? null

/** The cant [mm] at x along its ramp: straight, or 3t² − 2t³ on a curved one. */
function cantAt(ramps, x) {
  const r = rampAt(ramps, x)
  if (!r) return null
  const t = (x - r.from) / (r.to - r.from)
  return r.u0 + (r.u1 - r.u0) * (r.curved ? t * t * (3 - 2 * t) : t)
}

/** The slope of the roll angle along the track, dφ/dx [rad/m], at x; 0 beyond it, where the cant is taken to stay as it ends. */
function rollSlope(ramps, x) {
  const r = rampAt(ramps, x)
  if (!r) return 0
  const len = r.to - r.from
  const t = (x - r.from) / len
  const du = (r.u1 - r.u0) * (r.curved ? 6 * t * (1 - t) : 1) / len / 1000   // du/dx [m/m]
  const u = cantAt(ramps, x) / 1000
  return du / Math.sqrt(Math.max(1e-12, RUNNING_CIRCLES * RUNNING_CIRCLES - u * u))
}

/** How finely the band is read where a curved ramp makes the roll change all along [m]. */
const CURVED_STEP = 1

/**
 * Whether the band can be drawn: every element states its speed, and every
 * arc its cant — a straight without one has none, a transition takes its
 * ends from its neighbours. The curvature is the geometry's own.
 */
export function lateralInputsComplete(elements) {
  if (!elements?.length) return false
  return elements.every(el => {
    if (!(el.speed > 0)) return false
    if (el.elementType !== 'arc' || !el.radius) return true
    return [el.cant, el.cantStart, el.cantEnd].some(v => v != null)
  })
}

/**
 * The band [m/s²] over the track: { s, v } points out of its curvature
 * [1/km], cant [mm] and speed [km/h] bands and the elements' spans
 * (elementSpans), as bandSpecs reads them — two points at one station where
 * it steps. null where an input is missing (lateralInputsComplete).
 */
export function lateralAccelerationBand(elements, { curvature, cant, speed, spans }, vehicle = REFERENCE_VEHICLE) {
  if (!lateralInputsComplete(elements) || !curvature.length) return null
  const length = curvature[curvature.length - 1].s
  const half = vehicle.base / 2
  const ramps = rampsOf(spans, cant, curvature)
  // Where the band may bend or step: every point of its inputs, half the
  // vehicle on either side of every element joint, where the roll's average
  // takes in a ramp's end or lets it go, and a fine grid wherever a curved
  // ramp is within half a vehicle.
  const at = new Set([...curvature, ...speed, ...cant].map(p => p.s))
  for (const r of ramps) {
    for (const x of [r.from - half, r.from + half, r.to - half, r.to + half]) if (x > 0 && x < length) at.add(x)
    if (r.curved && r.u1 !== r.u0) {
      for (let x = Math.max(0, r.from - half); x <= Math.min(length, r.to + half); x += CURVED_STEP) at.add(x)
    }
  }
  const stations = [...at].map(s => Math.round(s * 1e6) / 1e6).sort((a, b) => a - b)
    .filter((s, i, all) => i === 0 || s > all[i - 1])

  const value = (x) => {
    const v = valueAt(speed, x), k = curvatureAt(ramps, curvature, x), u = cantAt(ramps, x)
    if (v == null || k == null || u == null) return null
    const ms = v / 3.6
    const phi = rollOf(u)
    const roll = ms * ms * (rollSlope(ramps, x + half) - rollSlope(ramps, x - half)) / vehicle.base
    return ms * ms * (k / 1000) * Math.cos(phi) - G * Math.sin(phi) + vehicle.height * roll
  }
  // Each stretch between two stations by its values just inside its ends:
  // a step at a station shows as two points there.
  const out = []
  for (let i = 1; i < stations.length; i++) {
    const a = stations[i - 1], b = stations[i]
    const e = Math.min(1e-6, (b - a) / 4)
    const first = { s: a, v: value(a + e) }
    const last = out[out.length - 1]
    if (!last || last.s !== a || last.v !== first.v) out.push(first)
    out.push({ s: b, v: value(b - e) })
  }
  return out
}
