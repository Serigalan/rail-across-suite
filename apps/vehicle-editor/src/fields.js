/**
 * The values of a vehicle the editor shows, in the units OSRD's editor shows
 * them: kept in SI as OSRD keeps them, shown scaled — the mass in t, speeds in
 * km/h, the resistance in kN, kN/(km/h) and kN/(km/h)². `path` where the value
 * is kept, `scale` what it is multiplied by to be shown.
 */

export const DYNAMICS = [
  { path: 'length', unit: 'm' },
  { path: 'mass', unit: 't', scale: 1 / 1000 },
  { path: 'max_speed', unit: 'km/h', scale: 3.6 },
  { path: 'inertia_coefficient', unit: '' },
  { path: 'const_gamma', unit: 'm/s²' },
  { path: 'startup_time', unit: 's' },
  { path: 'startup_acceleration', unit: 'm/s²' },
  { path: 'comfort_acceleration', unit: 'm/s²' },
]

// N/(m/s) · v[m/s] = N/(m/s) / 3.6 · v[km/h]; kN = N / 1000
export const RESISTANCE = [
  { path: 'rolling_resistance.A', unit: 'kN', scale: 1 / 1000 },
  { path: 'rolling_resistance.B', unit: 'kN/(km/h)', scale: 1 / 3600 },
  { path: 'rolling_resistance.C', unit: 'kN/(km/h)²', scale: 1 / 12960 },
]

export const ELECTRIC = [
  { path: 'electrical_power_startup_time', unit: 's' },
  { path: 'raise_pantograph_time', unit: 's' },
]

export const valueAt = (v, path) => path.split('.').reduce((o, k) => o?.[k], v)

/** The vehicle with the value at `path` set — a copy along the path, the rest shared. */
export function withValue(v, path, value) {
  const [k, ...rest] = path.split('.')
  return { ...v, [k]: rest.length ? withValue(v[k] ?? {}, rest.join('.'), value) : value }
}

/** A kept value as shown: scaled, without the float's noise. */
export function shown(value, scale = 1) {
  if (value == null || !Number.isFinite(value)) return null
  return Number((value * scale).toPrecision(8))
}

/** A value as typed, as kept; null stays null. */
export const kept = (value, scale = 1) => (value == null ? null : value / scale)

/** A number for reading, in the language's way, with as many digits as it needs up to `digits` significant ones. */
export function formatNumber(value, language, digits = 4) {
  if (value == null || !Number.isFinite(value)) return '–'
  if (value === 0) return '0'
  const magnitude = Math.floor(Math.log10(Math.abs(value)))
  return value.toLocaleString(language, { maximumFractionDigits: Math.max(0, Math.min(8, digits - 1 - magnitude)) })
}

/** A vehicle in a line: mass, top speed, length. */
export const vehicleLine = (v, language) =>
  `${Math.round(v.mass / 1000).toLocaleString(language)} t · ${Math.round(v.max_speed * 3.6)} km/h · ${Math.round(v.length).toLocaleString(language)} m`

/**
 * Pasted rows of speed [km/h] and effort [kN] — from a spreadsheet (tabs), a
 * CSV (semicolons, or commas between numbers with points), or a text
 * (spaces) — as a curve in m/s and N; rows without two numbers are skipped.
 */
export function parseRows(text) {
  const speeds = [], efforts = []
  for (const line of String(text).split(/\r?\n/)) {
    const s = line.trim()
    if (!s) continue
    const cells = /[\t;]/.test(s) ? s.split(/[\t;]+/) : s.includes('.') && s.includes(',') ? s.split(',') : s.split(/\s+/)
    const [v, f] = cells.map(c => Number(c.trim().replace(',', '.')))
    if (Number.isFinite(v) && Number.isFinite(f)) { speeds.push(v / 3.6); efforts.push(f * 1000) }
  }
  return { speeds, max_efforts: efforts }
}
