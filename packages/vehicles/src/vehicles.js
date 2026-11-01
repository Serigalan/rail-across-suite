/**
 * Vehicles as OSRD has them: its RailJSON rolling stock — the physics a
 * running time needs (mass, length, top speed, the resistance to movement
 * A + B·v + C·v² in N with v in m/s, the service braking deceleration, the
 * tractive effort over speed per traction mode and comfort) and what names
 * and sorts a vehicle. The suite keeps them in this form (decision 0014), so
 * they go to OSRD and come from it as they are; what the tool does not edit
 * is kept along.
 */

export const RAILJSON_VERSION = '3.4'

/** OSRD's train categories (TrainMainCategory). */
export const CATEGORIES = [
  'HIGH_SPEED_TRAIN', 'INTERCITY_TRAIN', 'REGIONAL_TRAIN', 'NIGHT_TRAIN', 'COMMUTER_TRAIN',
  'FREIGHT_TRAIN', 'FAST_FREIGHT_TRAIN', 'TRAM_TRAIN', 'TOURISTIC_TRAIN', 'WORK_TRAIN',
]

/** OSRD's loading gauges (LoadingGaugeType). */
export const LOADING_GAUGES = ['G1', 'G2', 'GA', 'GB', 'GB1', 'GC', 'FR3.3', 'FR3.3/GB/G2', 'GLOTT']

/** The comforts a curve may be for, the first the one without heating or cooling. */
export const COMFORTS = ['STANDARD', 'AIR_CONDITIONING', 'HEATING']

/** Traction modes offered for a new curve set: OSRD names a mode freely, an electric one by its voltage. */
export const MODES = ['thermal', '15000V', '25000V', '1500V', '3000V', '750V']

/** The fields of a vehicle's metadata, as OSRD has them. */
export const METADATA = ['reference', 'series', 'subseries', 'family', 'grouping', 'type', 'unit', 'number', 'detail']

export const isElectricMode = (mode) => mode !== 'thermal' && /^\d+V$/.test(mode)
export const isElectric = (v) => Object.values(v.effort_curves?.modes ?? {}).some(m => m.is_electric)

const kmh = (v) => v / 3.6

/**
 * A tractive effort curve from the starting effort and the power at the rim:
 * the effort until the power runs out, then the power over the speed — at
 * every `step` km/h up to the top speed. { speeds [m/s], max_efforts [N] }.
 */
export function curveFromPower({ effort, power, maxSpeed, step = 10 }) {
  const corner = power / effort
  const speeds = []
  for (let v = 0; v < maxSpeed - 1e-9; v += kmh(step)) speeds.push(v)
  if (corner > 0 && corner < maxSpeed && !speeds.some(s => Math.abs(s - corner) < 0.5)) speeds.push(corner)
  speeds.push(maxSpeed)
  speeds.sort((a, b) => a - b)
  return { speeds, max_efforts: speeds.map(v => (v <= corner ? effort : power / v)) }
}

/** A new vehicle: a regional multiple unit of 120 t, 2 MW and 160 km/h to start from. */
export function newVehicle(name = '') {
  const maxSpeed = kmh(160)
  const curve = curveFromPower({ effort: 160_000, power: 2_000_000, maxSpeed })
  return {
    railjson_version: RAILJSON_VERSION,
    name,
    metadata: Object.fromEntries(METADATA.map(k => [k, ''])),
    length: 70, mass: 120_000, max_speed: maxSpeed,
    startup_time: 10, startup_acceleration: 0.05, comfort_acceleration: 0.25,
    const_gamma: 0.6, inertia_coefficient: 1.08,
    rolling_resistance: { type: 'davis', A: 1500, B: 30, C: 5.5 },
    effort_curves: {
      default_mode: '15000V',
      modes: { '15000V': { is_electric: true, default_curve: curve, curves: [{ cond: { comfort: 'STANDARD', electrical_profile_level: null, power_restriction_code: null }, curve }] } },
    },
    base_power_class: null, loading_gauge: 'G1', power_restrictions: {}, energy_sources: [],
    electrical_power_startup_time: 5, raise_pantograph_time: 15,
    supported_signaling_systems: [],
    primary_category: 'REGIONAL_TRAIN', other_categories: [],
  }
}

/** What an exported or kept rolling stock carries that is not the vehicle: OSRD's database fields, ours. */
const NOT_THE_VEHICLE = ['id', 'locked', 'version', 'liveries']

/**
 * A rolling stock in OSRD's RailJSON taken in as a vehicle: { vehicle, notes }.
 * Missing values get OSRD's defaults. A rolling resistance whose coefficients
 * are all below one is taken as newtons per kilogram — OSRD's example vehicles
 * have it so, which as newtons for the whole train is next to none — and
 * multiplied by the mass (note `resistance_per_kg`). Throws { code } for what is
 * not a rolling stock.
 */
export function fromOsrd(json) {
  if (json == null || typeof json !== 'object' || Array.isArray(json) || !json.effort_curves || !Number.isFinite(json.mass)) {
    throw Object.assign(new Error('not_rolling_stock'), { code: 'not_rolling_stock' })
  }
  const vehicle = structuredClone(json)
  for (const k of NOT_THE_VEHICLE) delete vehicle[k]
  const notes = []
  vehicle.railjson_version ??= RAILJSON_VERSION
  vehicle.name = String(vehicle.name ?? '').trim()
  vehicle.metadata = { ...Object.fromEntries(METADATA.map(k => [k, ''])), ...(vehicle.metadata ?? {}) }
  vehicle.startup_time ??= 0
  vehicle.startup_acceleration ??= 0
  vehicle.comfort_acceleration ??= 0
  vehicle.inertia_coefficient ??= 1
  vehicle.loading_gauge ??= 'G1'
  vehicle.other_categories ??= []
  vehicle.power_restrictions ??= {}
  vehicle.energy_sources ??= []
  vehicle.supported_signaling_systems ??= []
  const rr = { type: 'davis', A: 0, B: 0, C: 0, ...(vehicle.rolling_resistance ?? {}) }
  const { A, B, C } = rr
  if ([A, B, C].every(x => Number.isFinite(x) && x >= 0 && x < 1) && A + B + C > 0) {
    Object.assign(rr, { A: A * vehicle.mass, B: B * vehicle.mass, C: C * vehicle.mass })
    notes.push('resistance_per_kg')
  }
  vehicle.rolling_resistance = rr
  return { vehicle, notes }
}

/** A vehicle as OSRD takes it: its RailJSON rolling stock. */
export function toOsrd(vehicle) {
  const out = structuredClone(vehicle)
  for (const k of NOT_THE_VEHICLE) delete out[k]
  out.railjson_version = RAILJSON_VERSION
  return out
}

/** The curves of a mode, each with what it is for: [{ comfort, profile, restriction, curve, index }]. */
export function curvesOf(vehicle, mode) {
  return (vehicle.effort_curves?.modes?.[mode]?.curves ?? []).map((c, index) => ({
    comfort: c.cond?.comfort ?? null,
    profile: c.cond?.electrical_profile_level ?? null,
    restriction: c.cond?.power_restriction_code ?? null,
    curve: c.curve,
    index,
  }))
}

/**
 * The curve a train runs with in a mode and comfort, as OSRD chooses it
 * without electrical profiles and power restrictions: the one for that
 * comfort, else the one for the standard comfort, else the mode's default.
 */
export function curveFor(vehicle, mode = vehicle.effort_curves?.default_mode, comfort = 'STANDARD') {
  const m = vehicle.effort_curves?.modes?.[mode] ?? vehicle.effort_curves?.modes?.[vehicle.effort_curves?.default_mode]
  if (!m) return null
  const plain = curvesOf({ effort_curves: { modes: { m } } }, 'm').filter(c => c.profile == null && c.restriction == null)
  return (plain.find(c => c.comfort === comfort) ?? plain.find(c => c.comfort === 'STANDARD' || c.comfort == null))?.curve ?? m.default_curve
}

/** The comforts a mode has curves for. */
export const comfortsOf = (vehicle, mode) => COMFORTS.filter(c => curvesOf(vehicle, mode).some(x => x.comfort === c && x.profile == null && x.restriction == null))

/** The resistance to movement at a speed [m/s], in N. */
export function resistanceAt(vehicle, v) {
  const { A = 0, B = 0, C = 0 } = vehicle.rolling_resistance ?? {}
  return A + B * v + C * v * v
}

/** The highest power at the rim a curve has [W]. */
export const powerOf = (curve) => Math.max(0, ...(curve?.speeds ?? []).map((s, i) => s * curve.max_efforts[i]))

/** A vehicle as the simulation takes it, running in a mode and comfort. */
export function rollingStockOf(vehicle, { mode, comfort } = {}) {
  const curve = curveFor(vehicle, mode, comfort)
  return {
    mass: vehicle.mass,
    length: vehicle.length,
    max_speed: vehicle.max_speed,
    inertia_coefficient: vehicle.inertia_coefficient,
    rolling_resistance: { A: vehicle.rolling_resistance.A, B: vehicle.rolling_resistance.B, C: vehicle.rolling_resistance.C },
    const_gamma: vehicle.const_gamma,
    effort_curve: curve.speeds.map((speed, i) => ({ speed, max_effort: curve.max_efforts[i] })),
  }
}

const finite = (x) => typeof x === 'number' && Number.isFinite(x)

/** What is wrong with a curve, as a problem key, or null. */
export function curveProblem(curve) {
  if (!curve || !Array.isArray(curve.speeds) || curve.speeds.length < 2) return 'curve_points'
  if (curve.speeds.length !== curve.max_efforts?.length) return 'curve_points'
  if (!curve.speeds.every(finite) || !curve.max_efforts.every(finite)) return 'curve_empty'
  if (curve.speeds[0] !== 0) return 'curve_from_zero'
  if (!curve.speeds.every((s, i) => i === 0 || s > curve.speeds[i - 1])) return 'curve_order'
  if (curve.max_efforts.some(e => e < 0)) return 'curve_negative'
  return null
}

/**
 * What keeps a vehicle from being saved or simulated: [{ field, key }] —
 * `field` the path of the value (`mass`, `effort_curves.15000V.0`), `key` the
 * text that says what. The server checks the same, but for the order of a
 * curve's start; the simulation needs a curve from standstill.
 */
export function problemsOf(v) {
  const out = []
  const need = (ok, field, key) => { if (!ok) out.push({ field, key }) }
  need(String(v.name ?? '').trim() !== '', 'name', 'name_required')
  for (const field of ['length', 'mass', 'max_speed', 'const_gamma']) need(finite(v[field]) && v[field] > 0, field, 'positive')
  need(finite(v.inertia_coefficient) && v.inertia_coefficient >= 1, 'inertia_coefficient', 'inertia_min')
  for (const k of ['A', 'B', 'C']) need(finite(v.rolling_resistance?.[k]) && v.rolling_resistance[k] >= 0, `rolling_resistance.${k}`, 'not_negative')
  for (const field of ['startup_time', 'startup_acceleration', 'comfort_acceleration']) need(v[field] == null || (finite(v[field]) && v[field] >= 0), field, 'not_negative')
  need(CATEGORIES.includes(v.primary_category), 'primary_category', 'category_required')
  const modes = Object.entries(v.effort_curves?.modes ?? {})
  need(modes.length > 0, 'effort_curves', 'mode_required')
  need(!modes.length || v.effort_curves.modes[v.effort_curves.default_mode] != null, 'effort_curves', 'default_mode')
  for (const [mode, m] of modes) {
    const p = curveProblem(m.default_curve)
    if (p) out.push({ field: `effort_curves.${mode}.default`, key: p })
    for (const c of curvesOf(v, mode)) {
      const q = curveProblem(c.curve)
      if (q) out.push({ field: `effort_curves.${mode}.${c.index}`, key: q })
    }
  }
  return out
}

/**
 * A mode's curve changed — `index` its place among the mode's curves, or
 * 'default' for the mode's default curve. The curve for the standard comfort
 * and the default curve are one: OSRD has both, and the editor keeps them the
 * same.
 */
export function withCurve(vehicle, mode, index, curve) {
  const v = structuredClone(vehicle)
  const m = v.effort_curves.modes[mode]
  if (index === 'default') {
    m.default_curve = curve
    return v
  }
  m.curves[index] = { ...m.curves[index], curve }
  const cond = m.curves[index].cond ?? {}
  if ((cond.comfort === 'STANDARD' || cond.comfort == null) && cond.electrical_profile_level == null && cond.power_restriction_code == null) m.default_curve = curve
  return v
}

/** A traction mode added, its curves copied from the mode shown or made from the vehicle's power. */
export function withMode(vehicle, mode, from = null) {
  const v = structuredClone(vehicle)
  const source = from ? v.effort_curves.modes[from] : null
  const curve = source ? structuredClone(source.default_curve) : curveFromPower({ effort: 160_000, power: 2_000_000, maxSpeed: v.max_speed })
  v.effort_curves.modes[mode] = {
    is_electric: isElectricMode(mode),
    default_curve: curve,
    curves: source ? structuredClone(source.curves) : [{ cond: { comfort: 'STANDARD', electrical_profile_level: null, power_restriction_code: null }, curve }],
  }
  if (!v.effort_curves.modes[v.effort_curves.default_mode]) v.effort_curves.default_mode = mode
  if (isElectric(v)) {
    v.electrical_power_startup_time ??= 5
    v.raise_pantograph_time ??= 15
  }
  return v
}

/** A traction mode taken away; the default moves to another. */
export function withoutMode(vehicle, mode) {
  const v = structuredClone(vehicle)
  delete v.effort_curves.modes[mode]
  if (v.effort_curves.default_mode === mode) v.effort_curves.default_mode = Object.keys(v.effort_curves.modes)[0] ?? ''
  if (!isElectric(v)) {
    v.electrical_power_startup_time = null
    v.raise_pantograph_time = null
  }
  return v
}

/** A curve for a comfort added to a mode, a copy of its standard one. */
export function withComfort(vehicle, mode, comfort) {
  const v = structuredClone(vehicle)
  const m = v.effort_curves.modes[mode]
  m.curves.push({ cond: { comfort, electrical_profile_level: null, power_restriction_code: null }, curve: structuredClone(curveFor(v, mode, 'STANDARD')) })
  return v
}

/** A mode's curve taken away. */
export function withoutCurve(vehicle, mode, index) {
  const v = structuredClone(vehicle)
  v.effort_curves.modes[mode].curves.splice(index, 1)
  return v
}

/** A traction mode in words — 15 kV 16,7 Hz, 1,5 kV =, thermisch — by the texts' `mode_<mode>`, else as OSRD names it. */
export function modeLabel(t, mode) {
  const key = `mode_${mode}`
  const said = t(key)
  return said === key ? mode : said
}
