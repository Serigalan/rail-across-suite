import { niceStep, stepDecimals, ticks } from '../common'

/**
 * One band of a track (Trassenbänder: curvature, cant, height plan) as an SVG
 * drawing to scale, in millimetres — the way the speed ranges give theirs
 * (speedRanges.js): its line over the station, the thinner line under it
 * where it has one, the element boundaries dashed, the labels where they
 * have room, the value axis and the station along the bottom.
 */

/** How many millimetres a kilometre of the track is drawn, unless another is given (1:10 000). */
export const BAND_SVG_MM_PER_KM = 100

/** How many millimetres one unit of each band is high, unless another is given. */
export const BAND_SVG_MM_PER_UNIT = { curvature: 10, cant: 0.2, height: 2, speed: 0.5 }

/** A label is left out where its stretch is narrower than this [mm]. */
const LABEL_MIN_MM = 9

const xml = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
const n = (v) => Math.round(v * 1000) / 1000

/** The values of a band's points between two stations, the line's ends there included. */
function valuesIn(points, from, to) {
  const out = []
  for (let i = 0; i < points.length; i++) {
    const p = points[i]
    if (p.v == null) continue
    if (p.s >= from && p.s <= to) out.push(p.v)
    const q = points[i + 1]
    // a segment that crosses an end of the range: its value there
    if (q && q.v != null && q.s > p.s) {
      for (const at of [from, to]) {
        if (p.s < at && q.s > at) out.push(p.v + (q.v - p.v) * (at - p.s) / (q.s - p.s))
      }
    }
  }
  return out
}

/**
 * The frame of a band's drawing [mm]: the stretch (`from`–`to` [m]), its
 * value range (`lo`–`hi`, by the band's own `scaleOf` over the values in
 * that stretch), the plot inside the margins — to scale both ways —, the steps of both axes, the
 * width `W` and height `H`.
 */
export function bandSvgSize(spec, { from, to }, { mmPerKm = BAND_SVG_MM_PER_KM, mmPerUnit }) {
  const values = [...valuesIn(spec.points, from, to), ...valuesIn(spec.under ?? [], from, to)]
  const range = values.length ? [Math.min(...values), Math.max(...values)] : null
  const [lo, hi] = spec.scaleOf(range)
  const left = 26, right = 12, top = 20, bottom = 16
  // the plot to scale; the frame no narrower than its title needs
  const plotW = ((to - from) / 1000) * mmPerKm
  const plotH = Math.max((hi - lo) * mmPerUnit, 10)
  const valueStep = niceStep(4, plotH / (hi - lo))
  const stationStep = niceStep(18, mmPerKm / 1000)
  return { from, to, lo, hi, left, top, plotW, plotH, valueStep, stationStep, mmPerKm, W: left + Math.max(plotW, 60) + right, H: top + plotH + bottom }
}

/** A polyline through points, broken where a value is null. */
function linePath(points, x, y) {
  let d = ''
  let pen = false
  for (const p of points) {
    if (p.v == null) { pen = false; continue }
    d += `${pen ? 'L' : 'M'}${x(p.s)},${y(p.v)}`
    pen = true
  }
  return d
}

/**
 * The band's drawing: `spec` as bandSpecs gives it ({ points, under, runs,
 * signed, unit }), `boundaries` the stations where one element ends and the
 * next begins, `range` the stretch drawn, `scale` { mmPerKm, mmPerUnit },
 * `texts` { title, subtitle, station, note }.
 */
export function bandSvg(spec, { boundaries = [], range, scale, texts }) {
  const size = bandSvgSize(spec, range, scale)
  const { from, to, lo, hi, left, top, plotW, plotH, valueStep, stationStep, W, H } = size
  const x = (s) => n(left + (s - from) / 1000 * size.mmPerKm)
  const y = (v) => n(top + plotH - (v - lo) / (hi - lo) * plotH)
  const right = n(left + plotW)
  const base = n(top + plotH)
  const out = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${n(W)}mm" height="${n(H)}mm" viewBox="0 0 ${n(W)} ${n(H)}" font-family="sans-serif" font-size="2.8">`,
    `<defs><clipPath id="plot"><rect x="${left}" y="${top}" width="${n(plotW)}" height="${n(plotH)}"/></clipPath></defs>`,
    `<rect width="${n(W)}" height="${n(H)}" fill="#fff"/>`,
    `<text x="${left}" y="8" font-size="4.2" font-weight="bold">${xml(texts.title)} <tspan font-weight="normal" fill="#666">[${xml(spec.unit)}]</tspan></text>`,
    `<text x="${left}" y="13" fill="#444">${xml([texts.subtitle, texts.note].filter(Boolean).join(' · '))}</text>`,
  ]
  const stations = ticks(from, to, stationStep)
  for (const s of stations) out.push(`<line x1="${x(s)}" y1="${top}" x2="${x(s)}" y2="${base}" stroke="#e5e5e5" stroke-width="0.2"/>`)
  for (const v of ticks(lo, hi, valueStep)) {
    out.push(`<line x1="${n(left - 1.2)}" y1="${y(v)}" x2="${left}" y2="${y(v)}" stroke="#333" stroke-width="0.25"/>`,
      `<text x="${n(left - 2)}" y="${n(y(v) + 1)}" text-anchor="end" fill="#444">${v.toFixed(stepDecimals(valueStep))}</text>`)
  }
  out.push('<g clip-path="url(#plot)">')
  for (const s of boundaries) {
    if (s > from && s < to) out.push(`<line x1="${x(s)}" y1="${top}" x2="${x(s)}" y2="${base}" stroke="#bbb" stroke-width="0.15" stroke-dasharray="1 1"/>`)
  }
  if (lo < 0 && hi > 0) out.push(`<line x1="${left}" y1="${y(0)}" x2="${right}" y2="${y(0)}" stroke="#333" stroke-width="0.25"/>`)
  if (spec.under?.length > 1) out.push(`<path d="${linePath(spec.under, x, y)}" fill="none" stroke="#999" stroke-width="0.25"/>`)
  const line = linePath(spec.points, x, y)
  if (line) out.push(`<path d="${line}" fill="none" stroke="#303383" stroke-width="0.6" stroke-linejoin="round"/>`)
  // Each label in the middle of what is drawn of its stretch, above or below the line.
  for (const r of spec.runs ?? []) {
    const a = Math.max(r.from, from), b = Math.min(r.to, to)
    if (x(b) - x(a) < LABEL_MIN_MM) continue
    const s = (a + b) / 2
    const v = r.valueAt(s)
    if (v == null) continue
    const ly = Math.min(Math.max(v >= (lo + hi) / 2 ? y(v) + 3.6 : y(v) - 1.4, top + 3), base - 1)
    out.push(`<text x="${x(s)}" y="${n(ly)}" text-anchor="middle" font-size="2.4" fill="#333">${xml(r.text)}</text>`)
  }
  out.push('</g>')
  if (texts.empty) out.push(`<text x="${n(left + plotW / 2)}" y="${n(top + plotH / 2 + 1)}" text-anchor="middle" fill="#888">${xml(texts.empty)}</text>`)
  out.push(`<rect x="${left}" y="${top}" width="${n(plotW)}" height="${n(plotH)}" fill="none" stroke="#333" stroke-width="0.25"/>`)
  for (const s of stations) {
    out.push(`<line x1="${x(s)}" y1="${base}" x2="${x(s)}" y2="${n(base + 1.2)}" stroke="#333" stroke-width="0.25"/>`,
      `<text x="${x(s)}" y="${n(base + 4.5)}" text-anchor="middle" fill="#333">${s.toFixed(stepDecimals(stationStep))}</text>`)
  }
  out.push(`<text x="${right}" y="${n(H - 3)}" text-anchor="end" fill="#666">${xml(texts.station)} [m]</text>`, '</svg>')
  return out.join('\n') + '\n'
}
