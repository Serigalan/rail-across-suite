import { buildElement } from './trackPositionCsvImport'
import { elementTypeName, epsgForLagesystem, normLine } from './mdbImport'
import { mdbStation } from './mdbGradient'
import { clipRuns, kmLineId } from '../km'
import { kmDesignElement } from './vermEsnImport'

/**
 * The kilometrage line of the DB ASCII interface (Satzart 24) as the
 * `kmLines` a project keeps.
 *
 * A kilometrage element is laid out like a track element of Satzart 21 — start
 * point, bearing in gon, length, radii, the same type codes — and carries the
 * kilometrage at both its ends (`EKAKM`, `EKEKM`, packed like the stations of
 * Satzart 11: mdbStation). Along the element the kilometrage runs with the
 * length, so every vertex of its drawn line takes the share of the way it is
 * along. Type 6 is not geometry but the kilometrage jump between two
 * elements: its length is the jump (negative where the kilometrage runs
 * back), and a run of the line ends there and the next begins.
 *
 * Which line an element belongs to is read from its start point (Satzart 11
 * names the line of every point; those of the kilometrage line carry
 * Richtungskennzeichen 3). The file states each element once per Lagesystem
 * it was computed in; the first is taken.
 *
 * What comes back has the shape of the line files shipped with the app —
 * WGS84 runs of `[lon, lat, metres]` — so everything that reads a kilometrage
 * reads this one too, to the millimetre where the shipped lines have whole
 * metres.
 */

const GON2DEG = 0.9
/** The kilometrage jump between two elements: no geometry. */
const EKTYP_JUMP = 6
/** Two kilometrages closer than this continue each other [m]. */
const SAME_KM = 0.01

const padKey = (a, b) => `${a}\u0000${b}`
const round = (v, d) => Math.round(v * 10 ** d) / 10 ** d

/** Distance [m] between two WGS84 points, flat — the vertices are metres apart. */
function step([lon1, lat1], [lon2, lat2]) {
  const k = Math.cos(((lat1 + lat2) / 2) * Math.PI / 180)
  return Math.hypot((lon2 - lon1) * 111320 * k, (lat2 - lat1) * 110574)
}

/** One element laid out: its vertices with their kilometrage, the element in its plane and that plane, or null. */
function laidOut(el, coords) {
  const typ = elementTypeName(el.typ)
  const epsg = epsgForLagesystem(el.sys)
  const start = coords.get(padKey(el.pad1, el.sys))
  const akm = mdbStation(el.akm)
  const ekm = mdbStation(el.ekm)
  if (!typ || !epsg || !start || akm == null || ekm == null || !(Number(el.p1) > 0)) return null
  const built = buildElement({
    typ, start, bearing: Number(el.ariwi) * GON2DEG, length: Number(el.p1), p2: Number(el.p2), p3: Number(el.p3),
  }, epsg)
  const line = built.geometry?.coordinates ?? []
  if (line.length < 2) return null
  const along = [0]
  for (let i = 1; i < line.length; i++) along.push(along[i - 1] + step(line[i - 1], line[i]))
  const total = along[along.length - 1] || 1
  const vertices = line.map(([lon, lat], i) => [round(lon, 8), round(lat, 8), round(akm + (ekm - akm) * along[i] / total, 3)])
  return { vertices, element: kmDesignElement(built), epsg, akm, ekm }
}

/**
 * The lines of the file as runs, keyed by line number: Map(line → runs).
 * `lines` limits it to these line numbers (strings, as normLine spells them).
 * A Map given as `design` takes per line its design as core 6.0 keeps a km
 * line (decision 0006): `{ epsg, runs }`, each run with its kilometrage at the
 * start, the way it counts and its elements in their plane — none for a line
 * laid out in more than one plane.
 */
export function mdbKmRuns(payload, lines = null, { design = null } = {}) {
  const coords = new Map()
  for (const p of payload?.points ?? []) {
    if (Number.isFinite(p?.y) && Number.isFinite(p?.x)) coords.set(padKey(p.pad, p.sys), [p.y, p.x])
  }
  const lineOf = new Map()
  for (const s of payload?.stations ?? []) {
    if (s?.pad && !lineOf.has(s.pad)) lineOf.set(s.pad, normLine(s.strecke))
  }

  // Per line, the elements in file order with each pad pair once.
  const byLine = new Map()
  const seen = new Set()
  for (const el of payload?.km ?? []) {
    const line = lineOf.get(el?.pad1)
    if (!line || (lines && !lines.has(line))) continue
    const key = padKey(el.pad1, el.pad2)
    if (seen.has(key)) continue
    seen.add(key)
    if (!byLine.has(line)) byLine.set(line, [])
    byLine.get(line).push(el)
  }

  const out = new Map()
  for (const [line, elements] of byLine) {
    // Walk the chains the point addresses describe; a jump or a gap ends a run.
    const next = new Map()
    const into = new Set()
    for (const el of elements) {
      if (!next.has(el.pad1)) next.set(el.pad1, [])
      next.get(el.pad1).push(el)
      into.add(el.pad2)
    }
    const runs = []
    const designRuns = []
    const planes = new Set()
    const used = new Set()
    const walk = (first) => {
      let run = []
      let designRun = null
      const close = () => {
        if (run.length >= 2) { runs.push(run); if (designRun) designRuns.push(designRun) }
        run = []; designRun = null
      }
      for (let el = first; el && !used.has(el); el = next.get(el.pad2)?.find(e => !used.has(e))) {
        used.add(el)
        if (Number(el.typ) === EKTYP_JUMP) { close(); continue }
        const laid = laidOut(el, coords)
        if (!laid) { close(); continue }
        const v = laid.vertices
        const last = run[run.length - 1]
        if (last && Math.abs(last[2] - v[0][2]) > SAME_KM) close()
        run = run.length ? [...run, ...v.slice(1)] : v
        designRun ??= { kmStart: laid.akm, counting: laid.ekm < laid.akm ? 'down' : 'up', elements: [] }
        designRun.elements.push(laid.element)
        planes.add(laid.epsg)
      }
      close()
    }
    for (const el of elements) if (!into.has(el.pad1)) walk(el)
    for (const el of elements) if (!used.has(el)) walk(el)
    if (runs.length) out.set(line, runs)
    if (design && designRuns.length && planes.size === 1) design.set(line, { epsg: [...planes][0], runs: designRuns })
  }
  return out
}

/**
 * The `kmLines` entries for the lines `lines` names, cut to `box` (the
 * project's extent, projectBox) like the shipped ones. Each is marked
 * `source: 'mdb'`: it replaces the shipped line of that number in the
 * project, and ensureKmLines fetches nothing over it.
 */
export function mdbKmLines(payload, lines, box) {
  const wanted = new Set([...lines].map(normLine))
  const design = new Map()
  const out = []
  for (const [line, runs] of mdbKmRuns(payload, wanted, { design })) {
    const clipped = box ? clipRuns(runs, box) : runs
    if (!clipped.length) continue
    const lineNumber = /^\d+$/.test(line) ? Number(line) : line
    // The design keeps the runs the box touches whole: elements are not cut, and the door draws the line from them.
    const kept = design.get(line)
    const touched = kept ? kept.runs.filter((_, i) => !box || clipRuns([runs[i]], box).length) : []
    out.push({
      id: kmLineId(lineNumber), lineNumber, crs: 'EPSG:4326', clip: box, runs: clipped, source: 'mdb',
      ...(touched.length ? { design: { epsg: kept.epsg, runs: touched } } : {}),
    })
  }
  return out
}
