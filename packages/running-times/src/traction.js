/**
 * The traction a train runs with along its way, from the electrification of
 * the net (the layer `electrification`, decision 0025): which stretches of a
 * path lie under contact wire of which system, and the vehicle's mode to run
 * with — the one asked for, else the vehicle's electric mode that has wire the
 * whole way, else its thermal one, else its default. The simulation runs one
 * mode the whole way (OSRD's switching between modes is not ported); where the
 * mode chosen is electric and the wire is missing or of another system, the
 * stretches are told (`unpowered`).
 */

/** The ranges under wire along a path: [{ begin, end, system }] in running order, offsets along the path [m]. */
export function electrifiedAlong(record, path) {
  const ranges = record?.electrification?.collections?.electrifications ?? []
  const out = []
  let at = 0
  for (const p of path ?? []) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const along = (station) => at + Math.abs(station - p.start)
    for (const r of ranges) {
      if (r.at?.trackId !== p.trackId) continue
      const a = Math.max(lo, r.at.startStation), b = Math.min(hi, r.at.endStation)
      if (b - a <= 1e-6) continue
      const [begin, end] = [along(a), along(b)].sort((x, y) => x - y)
      out.push({ begin, end, system: r.system })
    }
    at += hi - lo
  }
  return out.sort((x, y) => x.begin - y.begin)
}

/** The stretches of [0, length] the ranges of `system` leave without wire: [{ begin, end }]. */
export function withoutWire(ranges, system, length) {
  const mine = ranges.filter(r => r.system === system).sort((x, y) => x.begin - y.begin)
  const out = []
  let at = 0
  for (const r of mine) {
    if (r.begin > at + 0.5) out.push({ begin: at, end: r.begin })
    at = Math.max(at, r.end)
  }
  if (length > at + 0.5) out.push({ begin: at, end: length })
  return out
}

/** Whether a vehicle's mode is electric: OSRD's flag, else its name a voltage. */
const electric = (vehicle, mode) => vehicle?.effort_curves?.modes?.[mode]?.is_electric ?? /^\d+V$/.test(mode ?? '')

/**
 * The mode a vehicle runs a path with: { mode, auto, unpowered } — `auto`
 * whether it was chosen here, `unpowered` the stretches without the mode's
 * wire where it is electric. `asked` the train's own mode, null for the
 * choice by the wire; `ranges` electrifiedAlong's, `length` the path's;
 * `planned` whether the net's electrification is planned at all — where it is
 * not, a vehicle runs as it always did, in its default mode.
 */
export function tractionFor(vehicle, asked, ranges, length, planned = true) {
  const modes = Object.keys(vehicle?.effort_curves?.modes ?? {})
  const fallback = vehicle?.effort_curves?.default_mode ?? modes[0] ?? null
  const unpowered = (mode) => (electric(vehicle, mode) ? withoutWire(ranges, mode, length) : [])
  if (!planned) return { mode: modes.includes(asked) ? asked : fallback, auto: false, unpowered: [] }
  if (asked && modes.includes(asked)) return { mode: asked, auto: false, unpowered: unpowered(asked) }
  const covering = modes.filter(m => electric(vehicle, m) && !withoutWire(ranges, m, length).length)
  const thermal = modes.find(m => !electric(vehicle, m))
  const mode = covering.includes(fallback) ? fallback : covering[0] ?? thermal ?? fallback
  return { mode, auto: true, unpowered: unpowered(mode) }
}
