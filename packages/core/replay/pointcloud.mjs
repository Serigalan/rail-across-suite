// The vectors of the point clouds (crates/pointcloud, replayed by its own
// tests/replay.rs): LAS files made here, the E57 fixtures of the package by
// path, files made from a description on both sides (points scattered in a
// box, a synthetic track scene); registration, plane mapping, slices, the
// clearance check and scan, rail detection and tracing held to the JavaScript.
// The readers' and the import's vectors are frozen: their JavaScript is gone
// (decision 0020, step 10), and the clouds the rest runs on are imported by
// the Rust module.
import { readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { importLevels } from '@rail-across/pointcloud/src/native.js'
import { loadPointCloudModuleFile } from '@rail-across/pointcloud/src/nativeNode.js'
import { planeMapper } from '@rail-across/pointcloud/src/cloudCrs.js'
import * as R from '@rail-across/pointcloud/src/registration.js'
import { sliceFrame, tilesInSlice } from '@rail-across/pointcloud/src/cloudSlice.js'
import { checkClearance } from '@rail-across/pointcloud/src/clearanceCheck.js'
import { scanClearance, stretchesOf } from '@rail-across/pointcloud/src/clearanceScan.js'
import { detectTrack, edgeCandidates, topSurface } from '@rail-across/pointcloud/src/railDetect.js'
import * as T from '@rail-across/pointcloud/src/railTrace.js'
import { cloudSectionPoints, forgetAllClouds } from '@rail-across/pointcloud/src/cloudSection.js'
import { makeLas, bytesSource } from '@rail-across/pointcloud/test/pointCloudFixture.js'
import { gaugeProfile, gaugeProfileAreas, gaugeProfileRing } from '../src/crossSection/index.js'
import { makeTrack } from '../src/track/index.js'
import { straightFrom } from '../src/geometry/index.js'

const here = dirname(fileURLToPath(import.meta.url))
const REPO = resolve(here, '../../..')

const clone = (v) => JSON.parse(JSON.stringify(v, (_, x) => (ArrayBuffer.isView(x) ? Array.from(x) : x)))
const bytesOf = (file) => (typeof file === 'string' ? new Uint8Array(Buffer.from(file, 'base64'))
  : file.make ? made(file) : new Uint8Array(readFileSync(join(REPO, file.path))))

/**
 * A file made from its description, the same way crates/pointcloud/src/ops.rs
 * makes it: points scattered in a box, or the track scene.
 */
function made(spec) {
  const opts = { offset: spec.offset ?? [0, 0, 0], scale: spec.scale ?? [0.001, 0.001, 0.001], format: spec.format ?? 1 }
  if (spec.make === 'track') return makeLas(trackScene(), opts)
  let pts = scatter(spec.n, spec.seed, { rgb: !!spec.rgb })
  if (spec.dark) pts = pts.map(p => ({ ...p, red: p.red & 0xFF, green: p.green & 0xFF, blue: 0 }))
  return makeLas(pts, opts)
}

async function call(op, fn, ...args) {
  const input = clone(args)
  try {
    return { op, input, output: clone(await fn(...args)) }
  } catch (err) {
    return { op, input, error: { message: err.message } }
  }
}

/** A deterministic noise source. */
function noise(seed = 1) {
  let s = seed
  return (amp) => {
    s = (s * 16807) % 2147483647
    return ((s / 2147483647) * 2 - 1) * amp
  }
}

const memoryWriter = () => {
  const parts = []
  let at = 0
  return {
    append(bytes) { const o = at; parts.push(bytes.slice()); at += bytes.length; return o },
    bytes() {
      const out = new Uint8Array(at)
      let o = 0
      for (const p of parts) { out.set(p, o); o += p.length }
      return out
    },
  }
}

/**
 * A file imported by the import module (crates/pointcloud-wasm) — the clouds
 * the JavaScript's sections, scans and traces below run on. The module took
 * the JavaScript import's place (decision 0020, step 10).
 */
function importInto(file, { levels = [1], idlePoints, from, to } = {}) {
  const source = bytesSource(bytesOf(file))
  const writers = Object.fromEntries(levels.map(l => [l, memoryWriter()]))
  const bodies = importLevels({ source, levels, writers, idlePoints, ...(from != null ? { sourceCrs: from, targetCrs: to } : {}) })
  return { bodies, writers }
}

let cloudNo = 0
/** A cloud as the Rust op builds it: the file imported at level 1 into memory. */
async function cloudOf(spec) {
  const { bodies, writers } = await importInto(spec.file, { levels: [1], idlePoints: spec.idlePoints })
  const bytes = writers[1].bytes()
  return {
    ...bodies[1], id: spec.id, server: { level: 1 }, crs: spec.crs ?? null, transform: spec.transform ?? null,
    source: { readMany: async (ranges) => ranges.map(([o, l]) => bytes.subarray(o, o + l)) },
  }
}
const spec = (file, extra = {}) => ({ file, id: `cloud-${++cloudNo}`, ...extra })

// ── the files ────────────────────────────────────────────────────────────────

function scatter(n, seed, { rgb = false, base = [500000, 5700000, 100], span = [30, 20, 5] } = {}) {
  const rnd = noise(seed)
  return Array.from({ length: n }, () => ({
    x: base[0] + span[0] / 2 + rnd(span[0] / 2), y: base[1] + span[1] / 2 + rnd(span[1] / 2), z: base[2] + span[2] / 2 + rnd(span[2] / 2),
    intensity: Math.floor(Math.abs(rnd(4000))),
    ...(rgb ? { red: Math.floor(Math.abs(rnd(65535))), green: Math.floor(Math.abs(rnd(65535))), blue: Math.floor(Math.abs(rnd(255))) } : {}),
  }))
}

/**
 * A straight track scene: two rail heads with their inner flanks, sleepers
 * and ballast, along `length` m from `start` at grid bearing `bearing`, in
 * DHDN-free UTM 32; a mast at station 7 standing in the clearance outline.
 */
function trackScene({ start = [500100, 5700100], bearing = 60, length = 12, z = 100 } = {}) {
  const rnd = noise(11)
  const b = bearing * Math.PI / 180
  const along = [Math.sin(b), Math.cos(b)], right = [Math.cos(b), -Math.sin(b)]
  const pts = []
  const at = (s, q, h, intensity) => pts.push({
    x: start[0] + along[0] * s + right[0] * q + rnd(0.001), y: start[1] + along[1] * s + right[1] * q + rnd(0.001),
    z: h + rnd(0.001), intensity,
  })
  const sleeper = z - 0.17
  for (let s = 0; s <= length; s += 0.02) {
    for (const side of [-1, 1]) {
      const inner = side * 0.7175, centre = inner + side * 0.0335
      for (let q = centre - 0.0335; q <= centre + 0.0335 + 1e-9; q += 0.01) at(s, q, z, 3000)
      for (let dz = 0.005; dz <= 0.05; dz += 0.004) at(s, inner, z - dz, 2500)
    }
  }
  for (let s = 0; s <= length; s += 0.08) {
    for (let q = -2.5; q <= 2.5; q += 0.08) {
      const d = Math.abs(q)
      at(s, q, d < 1.3 ? sleeper : sleeper - 0.05 - (d - 1.3) * 0.3, 1200)
    }
  }
  for (let h = 0; h < 5; h += 0.05) at(7, 2.0, z + h, 4000)
  return pts
}

function trackOf({ start = [500100, 5700100], bearing = 60, length = 12, z = 100 } = {}) {
  const el = straightFrom({ easting: start[0], northing: start[1], zone: 25832 }, bearing, length, { speed: 80, cant: 0 })
  return makeTrack({ name: 'scene', epsg: 25832 }, [el], 'scene', [{ station: 0, z }, { station: length, z }])
}

function trackSlice({ axis = 0, z = 100, cant = 0, head = 0.067, gauge = 1.435, top = 0.17, jitter = 0.002, extra = [] } = {}) {
  const pts = []
  const rnd = noise(7)
  const add = (y, zz) => pts.push([y + rnd(jitter), zz + rnd(jitter)])
  const sleeper = z - top
  for (let y = axis - 2.5; y <= axis + 2.5; y += 0.01) {
    const d = Math.abs(y - axis)
    add(y, d < 1.3 ? sleeper : sleeper - 0.05 - (d - 1.3) * 0.3)
  }
  for (const side of [-1, +1]) {
    const inner = axis + side * gauge / 2
    const centre = inner + side * head / 2
    const zTop = side < 0 ? z + cant : z
    for (let y = centre - head / 2; y <= centre + head / 2 + 1e-9; y += 0.005) {
      for (let along = 0; along < 5; along++) add(y, zTop - 0.002 * along / 5)
    }
    for (let dz = 0.005; dz <= 0.05; dz += 0.004) add(inner, zTop - dz)
  }
  for (const [y, zz] of extra) add(y, zz)
  const out = { count: pts.length, y: new Float32Array(pts.length), z: new Float64Array(pts.length) }
  pts.forEach(([y, zz], k) => { out.y[k] = y; out.z[k] = zz })
  return out
}

// ── the recordings ───────────────────────────────────────────────────────────

/** The files the recordings make. */
function files() {
  const colour = { make: 'scatter', n: 3000, seed: 3, rgb: true, format: 3, scale: [0.01, 0.01, 0.001] }
  return { colour }
}

/**
 * Vectors whose JavaScript is gone — the LAS, LAZ and E57 readers and the
 * import (decision 0020, step 10): kept as they were last recorded, the Rust
 * held to them from now on as to any other test.
 */
const frozen = (name) => JSON.parse(readFileSync(join(REPO, 'crates/pointcloud/tests/vectors', `${name}.json`), 'utf8'))

function registrations() {
  const out = []
  const E0 = 4467335, N0 = 5333806, H0 = 508
  const line = (bearing, radius, step, posts) => {
    const pts = []
    for (let s = 0; s <= 200 + 1e-9; s += step) {
      let e, n, b
      if (radius) {
        const turn = s / radius, b0 = bearing * Math.PI / 180
        e = E0 + radius * (Math.cos(b0) - Math.cos(b0 + turn)); n = N0 + radius * (Math.sin(b0 + turn) - Math.sin(b0)); b = bearing + turn * 180 / Math.PI
      } else {
        e = E0 + s * Math.sin(bearing * Math.PI / 180); n = N0 + s * Math.cos(bearing * Math.PI / 180); b = bearing
      }
      const r = [Math.cos(b * Math.PI / 180), -Math.sin(b * Math.PI / 180)]
      const h = H0 + s * 0.002
      pts.push({ bearing: b, side: 'left', p: [e - 0.75 * r[0], n - 0.75 * r[1], h] })
      pts.push({ bearing: b, side: 'right', p: [e + 0.75 * r[0], n + 0.75 * r[1], h] })
      if (posts && Math.abs(s % posts) < 1e-9) pts.push({ bearing: b, side: 'mast', p: [e + 3.5 * r[0], n + 3.5 * r[1], h + 6] })
    }
    return pts
  }
  const pairsFor = (points, truth, kinds, seed = 3) => {
    const rnd = noise(seed)
    const inv = R.invertMatrix(truth)
    return points.map((q, k) => {
      const kind = kinds(q, k)
      const b = q.bearing * Math.PI / 180
      const slide = kind === 'section' ? rnd(0.04) : 0
      const src = R.applyMatrix(inv, [q.p[0] + Math.sin(b) * slide, q.p[1] + Math.cos(b) * slide, q.p[2]])
      return { id: `P${k + 1}`, kind, bearing: q.bearing, ref: q.p, src: src.map(x => x + rnd(0.002)) }
    })
  }
  const TRUE = { tE: 0.3 * Math.sin(110 * Math.PI / 180), tN: 0.3 * Math.cos(110 * Math.PI / 180), tH: 0.05, kappa: 0.0008 }
  const m = R.transformMatrix({ tE: 1, tN: -2, tH: 0.5, kappa: 0.3, omega: 0.01, phi: -0.02, scale: 2e-5 }, [E0, N0, H0])
  out.push({ op: 'pointcloud.transformMatrix', input: [{ tE: 1, tN: -2, tH: 0.5, kappa: 0.3, omega: 0.01, phi: -0.02, scale: 2e-5 }, [E0, N0, H0]], output: m })
  out.push({ op: 'pointcloud.invertMatrix', input: [m], output: R.invertMatrix(m) })
  out.push({ op: 'pointcloud.chainMatrices', input: [m, R.invertMatrix(m)], output: R.chainMatrices(m, R.invertMatrix(m)) })
  out.push({ op: 'pointcloud.applyMatrix', input: [m, [E0 + 12.3, N0 - 4.5, H0 + 1]], output: R.applyMatrix(m, [E0 + 12.3, N0 - 4.5, H0 + 1]) })
  out.push({ op: 'pointcloud.symmetricEigen', input: [[[4, 1, 0], [1, 3, 1], [0, 1, 2]]], output: R.symmetricEigen([[4, 1, 0], [1, 3, 1], [0, 1, 2]]) })
  for (const b of [110, 20, 0, 90]) out.push({ op: 'pointcloud.mainBearing', input: [line(b, null, 50, 100).map(q => q.p)], output: R.mainBearing(line(b, null, 50, 100).map(q => q.p)) })
  const curve = line(110, 1200, 50, 1000)
  const masts = line(110, 1200, 100, 100).filter(q => q.side === 'mast')
  const all = [...curve, masts[1]]
  const centreOf = (pairs) => [0, 1, 2].map(i => pairs.reduce((a, p) => a + p.src[i], 0) / pairs.length)
  const truth = R.transformMatrix(TRUE, centreOf(pairsFor(all, R.IDENTITY, () => 'section')))
  const cases = [
    [pairsFor(all, truth, (q) => (q.side === 'mast' ? '3d' : 'section')), {}],
    [pairsFor(all, truth, () => '3d'), { tilts: true, scale: true }],
    [pairsFor(line(110, null, 50, 100), truth, () => 'section'), {}],
    [pairsFor(line(110, null, 50, 100), truth, () => 'section'), { start: truth }],
    // Not here: tilts on a straight line. The Jacobian by central differences
    // at coordinates of 5·10⁶ m carries the last digit of sin and cos, and
    // with the tilts hardly determined JavaScript and Rust part by 1/1000 of
    // the tilts' own standard deviation — not to the digit.
    [pairsFor(line(30, null, 100, 0).slice(0, 2), truth, () => 'section'), {}],
    [pairsFor(all, truth, () => '3d').map((p, k) => (k === 3 ? { ...p, src: [p.src[0] + 0.2, p.src[1], p.src[2]] } : k === 5 ? { ...p, on: false } : p)), {}],
    [[], {}],
    [pairsFor(line(110, null, 200, 0).slice(0, 1), truth, () => '3d'), {}],
  ]
  // Tilts on points that lie nearly in a line are poorly determined: the
  // adjustment carries the last digit of sin and cos up to 1e-7 there.
  for (const [pairs, opts, tol] of cases) {
    out.push({ op: 'pointcloud.solveRegistration', input: clone([pairs, opts]), output: clone(R.solveRegistration(pairs, opts)), ...(tol ? { tol } : opts.tilts ? { tol: 1e-6 } : {}) })
  }
  return out
}

function planes() {
  const out = []
  const pts = [[500000, 5700000], [500123.4, 5700456.7], [499750.2, 5699999.9], [512345.6, 5712345.6]]
  for (const [a, b] of [[25832, 25833], [25832, 25832], [25832, 5683], [5682, 25832]]) {
    const m = planeMapper(a, b)
    out.push({ op: 'pointcloud.planeMapper', input: [a, b, pts], output: pts.map(([e, n]) => m(e, n)) })
  }
  return out
}

async function slicesAndChecks() {
  const out = []
  for (const f of [{ easting: 10, northing: 20, bearing: 30, halfWidth: 3, thickness: 0.1 }, { easting: -5, northing: 7, bearing: 271.5, halfWidth: 1, thickness: 2 }]) {
    out.push({ op: 'pointcloud.sliceFrame', input: [f], output: sliceFrame(f) })
  }
  const index = { tileSize: 2, tiles: [] }
  for (let tx = -3; tx <= 8; tx++) for (let ty = 0; ty <= 12; ty++) index.tiles.push([tx, ty, [[tx * 100 + ty, 10, 5, 0]]])
  for (const f of [sliceFrame({ easting: 4, northing: 10, bearing: 45, halfWidth: 3, thickness: 0.5 }), sliceFrame({ easting: 0, northing: 0, bearing: 0, halfWidth: 5, thickness: 0.1 }), sliceFrame({ easting: 9, northing: 20, bearing: 100, halfWidth: 1.5, thickness: 4 })]) {
    out.push({ op: 'pointcloud.tilesInSlice', input: [index, f], output: tilesInSlice(index, f) })
  }
  const profile = gaugeProfile('hauptgleis')
  const ring = gaugeProfileRing(profile.points)
  const areas = gaugeProfileAreas(profile.einragungen)
  const rnd = noise(5)
  const rows = Array.from({ length: 400 }, () => [rnd(3500), 2500 + rnd(2700)])
  rows.push([0, 2000], [2600, 1000], [1650, 500], [2000, 500], [700, 20], [9000, 1000])
  const slice = (zTrack) => ({ count: rows.length, y: Float32Array.from(rows.map(([y]) => y / 1000)), z: Float64Array.from(rows.map(([, z]) => zTrack + z / 1000)) })
  for (const cant of [0, 150, -90]) {
    const pts = slice(531.5)
    out.push({ op: 'pointcloud.checkClearance', input: clone([pts, { zTrack: 531.5, cant, ring, areas }]), output: clone(checkClearance(pts, { zTrack: 531.5, cant, ring, areas })) })
  }
  const pts = slice(10)
  out.push({ op: 'pointcloud.checkClearance', input: clone([pts, { zTrack: 10, ring }]), output: clone(checkClearance(pts, { zTrack: 10, ring })) })
  const hits = [{ station: 1, inside: 2, depth: 10 }, { station: 1.5, inside: 1, depth: 30 }, { station: 2, inside: 4, depth: 5 }, { station: 5, inside: 1, depth: 1 }, { station: 5.7, inside: 3, depth: 9 }]
  out.push({ op: 'pointcloud.stretchesOf', input: [hits, 0.5], output: stretchesOf(hits, 0.5) })
  out.push({ op: 'pointcloud.stretchesOf', input: [hits, 1], output: stretchesOf(hits, 1) })
  return { out, ring, areas }
}

function detections() {
  const out = []
  const slices = [
    [trackSlice({ axis: 0.12, z: 100, cant: 0.05 }), { around: 0, window: 0.3, rail: '54E4' }],
    [trackSlice({ axis: -0.4 }), { around: 0, window: 1.5, rail: '60E2' }],
    [trackSlice({ axis: 0, gauge: 1.47 }), { around: 0, window: 0.3 }],
    [trackSlice({ axis: 0, extra: [...Array(30)].map((_, k) => [0.5 + k * 0.01, 100.3]) }), { around: 0.1, window: 0.5, rail: 'nope' }],
    [trackSlice({ axis: 2 }), { around: 0, window: 0.3 }],
    [{ count: 0, y: new Float32Array(0), z: new Float64Array(0) }, {}],
    [trackSlice({ head: 0.03 }), { window: 0.3 }],
  ]
  for (const [s, o] of slices) {
    out.push({ op: 'pointcloud.detectTrack', input: clone([s, o]), output: clone(detectTrack(s, o)) })
    out.push({ op: 'pointcloud.topSurface', input: clone([s, -1.5, 1.5]), output: clone(topSurface(s, -1.5, 1.5)) })
    for (const side of [1, -1]) out.push({ op: 'pointcloud.edgeCandidates', input: clone([s, -1.5, 1.5, 0.067, side]), output: clone(edgeCandidates(topSurface(s, -1.5, 1.5), -1.5, 1.5, 0.067, side)) })
  }
  const pts = [
    { station: 0, offset: 0.1, turn: 2, zLeft: 100.01, zRight: 100, cant: 0.01, gauge: 1.434, quality: 'good', easting: 1, northing: 2 },
    { station: 0.5, offset: 0.12, turn: 0, zLeft: 100.002, zRight: 100, cant: 0.002, gauge: 1.436, quality: 'doubtful', easting: 1.5, northing: 2.5 },
    { station: 1, offset: 0.13, turn: 0, zLeft: 99.9, zRight: 100, cant: -0.1, gauge: 1.435, quality: 'good', easting: 2.123456, northing: 3.98765 },
    { station: 6, offset: 0.2, turn: -3, zLeft: 100, zRight: 100.05, cant: -0.05, gauge: 1.44, quality: 'other', easting: 7, northing: 8 },
  ]
  for (const [list, s] of [[pts.slice(0, 1), 0.5], [pts.slice(0, 2), 1.5], [pts.slice(0, 3), 2], [pts, 6.5], [pts, 20], [[], 1]]) {
    out.push({ op: 'pointcloud.foresee', input: clone([list, s]), output: clone(T.foresee(list, s)) })
  }
  out.push({ op: 'pointcloud.gapsOf', input: [[0, 0.5, 1, 3, 3.5, 7], 0.5], output: T.gapsOf([0, 0.5, 1, 3, 3.5, 7], 0.5) })
  for (const p of pts) {
    for (const ref of ['lower', 'axis', 'left', 'right']) out.push({ op: 'pointcloud.soHeight', input: [p, ref], output: T.soHeight(p, ref) })
    out.push({ op: 'pointcloud.cantMm', input: [p], output: T.cantMm(p) })
  }
  out.push({ op: 'pointcloud.axisPointsCsv', input: [pts, {}], output: T.axisPointsCsv(pts) })
  out.push({ op: 'pointcloud.axisPointsCsv', input: [pts, { soReference: 'axis' }], output: T.axisPointsCsv(pts, { soReference: 'axis' }) })
  const stations = [-1, 0, 0.5, 3.3, 10, 14.142, 20, 31]
  for (const g of [{ vertices: [[0, 0], [10, 0], [10, 0], [10, 10]], epsg: 25832 }, { vertices: [[5, 5]], epsg: 5683 }, { vertices: [[0, 0], [-3, 4]], epsg: null }]) {
    const guide = T.lineGuide(g.vertices, g.epsg)
    out.push({ op: 'pointcloud.guide', input: [g, stations], output: clone({ epsg: guide.epsg, length: guide.length, window: guide.window, at: stations.map(s => guide.at(s)) }) })
  }
  const track = trackOf()
  const tg = T.trackGuide(track)
  out.push({ op: 'pointcloud.guide', input: clone([{ kind: 'track', track }, stations]), output: clone({ epsg: tg.epsg, length: tg.length, window: tg.window, at: stations.map(s => tg.at(s)) }) })
  return out
}

async function scenes(ring, areas, colour) {
  const out = []
  const file = { make: 'track', offset: [500000, 5700000, 0] }
  const track = trackOf()
  const b = 60 * Math.PI / 180
  const origin = (s, q = 0) => ({ easting: 500100 + Math.sin(b) * s + Math.cos(b) * q, northing: 5700100 + Math.cos(b) * s - Math.sin(b) * q })
  forgetAllClouds()
  const sections = [
    [spec(file, { crs: 25832 }), { origin: origin(3), bearing: 60, crs: 25832, halfWidth: 3, thickness: 0.1 }],
    [spec(file, { crs: 25832 }), { origin: origin(7.01, 0.3), bearing: 61, crs: 25832, halfWidth: 4, thickness: 0.5 }],
    [spec(colour, { crs: 25832 }), { origin: { easting: 500015, northing: 5700010 }, bearing: 10, crs: 25832, halfWidth: 8, thickness: 3 }],
    [spec(file, { crs: 25832, transform: { matrix: R.transformMatrix({ tE: 0.2, tN: -0.1, tH: 0.05, kappa: 0.001 }, [500100, 5700100, 100]), crs: 25832 } }), { origin: origin(3), bearing: 60, crs: 25832, halfWidth: 3, thickness: 0.1 }],
  ]
  for (const [s, o] of sections) {
    out.push(await call('pointcloud.cloudSectionPoints', async (sp, op) => {
      const p = await cloudSectionPoints(await cloudOf(sp), op)
      return { count: p.count, y: Array.from(p.y.subarray(0, p.count)), z: Array.from(p.z.subarray(0, p.count)), i: Array.from(p.i.subarray(0, p.count)), ...(p.rgb ? { rgb: Array.from(p.rgb.subarray(0, 3 * p.count)) } : {}), tiles: p.tiles }
    }, s, o))
  }
  for (const [s, o] of [
    [[spec(file, { crs: 25832 })], { origin: origin(4), bearing: 60, crs: 25832, around: 0, window: 1.5, rail: '54E4' }],
    [[spec(file, { crs: 25832 }), spec(file, { crs: 25832 })], { origin: origin(4, 0.6), bearing: 62, crs: 25832, around: -0.5, window: 0.3 }],
  ]) {
    out.push(await call('pointcloud.detectInClouds', async (sp, op) => T.detectInClouds(await Promise.all(sp.map(cloudOf)), op), s, o))
  }
  const scan = async (p) => {
    const progress = []
    const result = await scanClearance({ ...p, clouds: await Promise.all(p.clouds.map(cloudOf)), onProgress: (x) => progress.push(x) })
    return { result, progress }
  }
  out.push(await call('pointcloud.scanClearance', scan, { clouds: [spec(file, { crs: 25832 })], track, ring, areas, from: 0, to: 12 }))
  out.push(await call('pointcloud.scanClearance', scan, { clouds: [spec(file, { crs: 25832 })], track, ring, areas: [], from: 5, to: 9, step: 1 }))
  out.push(await call('pointcloud.scanClearance', scan, { clouds: [], track: { ...track, heights: [] }, ring, areas }))
  const trace = async (p) => {
    const progress = []
    const guide = p.guide.kind === 'track' ? T.trackGuide(p.guide.track) : T.lineGuide(p.guide.vertices, p.guide.epsg)
    const result = await T.traceTrack({ clouds: await Promise.all(p.clouds.map(cloudOf)), guide, rail: p.rail, step: p.step, onProgress: (x) => progress.push(x) })
    return { result, progress }
  }
  out.push(await call('pointcloud.traceTrack', trace, { clouds: [spec(file, { crs: 25832 })], guide: { kind: 'track', track }, rail: '54E4', step: 1 }))
  const o0 = origin(0, 0.8), o1 = origin(12, 1.4)
  out.push(await call('pointcloud.traceTrack', trace, { clouds: [spec(file, { crs: 25832 })], guide: { kind: 'line', vertices: [[o0.easting, o0.northing], [o1.easting, o1.northing]], epsg: 25832 }, step: 2 }))
  return out
}

export default async function record() {
  if (!loadPointCloudModuleFile()) throw new Error('the point cloud module is not built (npm run wasm in packages/pointcloud)')
  const { colour } = files()
  const { out: checks, ring, areas } = await slicesAndChecks()
  return {
    pointcloud: {
      readers: frozen('readers'),
      imports: frozen('imports'),
      registration: registrations(),
      planes: planes(),
      slices: checks,
      detection: detections(),
      scenes: await scenes(ring, areas, colour),
    },
  }
}
