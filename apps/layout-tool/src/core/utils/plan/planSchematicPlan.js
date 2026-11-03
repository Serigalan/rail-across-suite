import { PAPER_FORMATS, TITLE_COLUMN_MM, drawingArea } from './planExport'
import { path, text, line, circlePath, cullItems, textWidth, STYLE } from './planItems'
import { frameItems } from './planSheet'
import { fitSize, titleBlockFor } from './planTitleBlock'
import { kmAt, kmMarks, schematicLayout, schematicStrip, switchNumber, toPlane } from '@rail-across/core/schematic'
import { STATUSES, STATUS_COLOR } from '@rail-across/core/track'
import { format } from '../../locales/i18n'
import { formatNumber } from '../common'
import { kmForTrackPoint } from '../km'

export { switchNumber }

/**
 * The schematic overview as a plan: the strip from planSchematic cut into
 * sheets along the kilometrage, drawn with the same primitives and the same
 * sheet furniture as the site plan, so the preview, the SVG and the PDF come
 * out of the same backends.
 */

/** Horizontal scales of the overview: denominator → mm per metre. */
export const SCHEMATIC_SCALES = { '5000': 5000, '10000': 10000, '25000': 25000, '50000': 50000 }

const SIDE_PAD = 10     // clear space inside the frame at both ends of the strip [mm]
const TOP_BAND = 24     // band for the station names [mm]
const BOTTOM_BAND = 18  // band for the kilometrage [mm]
const MAX_GAP = 10      // lane spacing, as the DB symbols are drawn [mm]
const MIN_GAP = 3       // below this the lanes no longer read apart [mm]
// A platform fills the gap beside its track from 2/10 to 8/10 of the way
// across, its edge lines 1/10 in from its long sides.
const PLATFORM_NEAR = 0.2
const PLATFORM_FAR = 0.8
const PLATFORM_EDGE = 0.1
const WEDGE_LEG = 3     // legs of a switch's triangle at full spacing [mm]
const KM_POST_R = 0.8   // kilometre post circle [mm]
const KM_POST_TEXT = 5.6 // from the post to its figure [mm]
// A switch number sits in the sharp angle of its switch beyond the triangle,
// clear of both strokes, an arrow before it pointing at the switch.
const SWITCH_TEXT = 2.117 // 6 pt [mm]
const CAP_HEIGHT = 0.72   // Helvetica's capitals, in text sizes
const SWITCH_CLEAR = 0.75 // from the track to the number [mm]
const ARROW_W = 0.55      // the arrow's length and height, in text sizes
const ARROW_H = 0.6
const ARROW_SPACE = 0.3   // between arrow and number, in text sizes

const STROKE = {
  track: 0.5,
  connector: 0.5,
  platformEdge: 0.25,
  ruler: 0.25,
  bracket: 0.25,
}
const LEGEND_ROW = 5    // legend line spacing [mm]

const PRIO = { station: 1, joint: 1, km: 2, switch: 3, track: 4 }

const fmt = (v, decimals, comma) => formatNumber(v, { digits: decimals, comma })

/**
 * A switch number beside the triangle at (x, y) that opens along `sx` and
 * across `sy`: the arrow nearest the switch pointing back at it, the number
 * beyond, both off the track on the side the branch leaves to.
 */
function switchLabel(x, y, { sx, sy, off, t, color }) {
  const s = SWITCH_TEXT
  const cap = CAP_HEIGHT * s
  const base = sy > 0 ? y + SWITCH_CLEAR + cap : y - SWITCH_CLEAR
  const mid = base - cap / 2
  const tip = x + sx * off
  const back = tip + sx * ARROW_W * s
  const arrow = path([['M', tip, mid], ['L', back, mid - ARROW_H * s / 2], ['L', back, mid + ARROW_H * s / 2], ['Z']],
    { stroke: null, fill: color })
  return text(back + sx * ARROW_SPACE * s, base, [{ t }],
    { size: s, align: sx > 0 ? 'left' : 'right', color, prio: PRIO.switch, marks: [arrow] })
}

/** A polyline as path commands. */
const poly = (pts) => pts.map((p, i) => [i ? 'L' : 'M', p[0], p[1]])

/** Kilometre posts as often as the scale leaves room for their figures. */
const postStep = (scaleDen) => (scaleDen <= 5000 ? 500 : scaleDen <= 10000 ? 1000 : scaleDen <= 25000 ? 2000 : 5000)

/**
 * What the title block says about one sheet: the stretch of the reference it
 * shows, for the location sketch, and its kilometrage, where the project has
 * a line to read it off.
 */
function sheetContext(layout, xa, xb, titleBlock, texts, comma) {
  const out = {}
  const sketch = titleBlock?.sketch
  if (sketch) {
    const line = layout.axis.filter(q => q.x >= xa && q.x <= xb).map(q => toPlane(q.p, layout.epsg, sketch.epsg))
    if (line.length > 1) out.focus = { line }
  }
  if (layout.kmKnown && texts.range) {
    const a = Math.max(xa, layout.extent[0])
    const b = Math.min(xb, layout.extent[1])
    const [ka, kb] = [kmAt(layout.kmTable, a), kmAt(layout.kmTable, b)].map(k => fmt(k / 1000, 1, comma))
    out.range = format(texts.range, { a: ka, b: kb })
  }
  return out
}

/**
 * Build the overview.
 *
 * @param {object} o
 * @param {Array}  o.tracks, o.switches, o.platforms, o.kmLines   project data
 * @param {string} o.leadTrackId  reference track (default: the longest strand)
 * @param {string} o.paperKey
 * @param {number} o.scaleDen     horizontal scale denominator
 * @param {object} o.titleBlock   as buildPlan takes it
 * @param {object} o.show         { km, switches, trackNames, platforms }
 * @param {object} o.texts        { next, prev, legend, range, status: { existing, new, removal } }
 *                                — `{n}` = sheet number, `{a}`/`{b}` = km
 * @param {boolean} o.comma
 * @returns {{ pageW, pageH, sheets, fits }} — `fits` false when the lanes had to be packed tighter than they read
 */
export function buildSchematicPlan({
  tracks, switches = [], platforms = [], kmLines = [], leadTrackId = null,
  paperKey, scaleDen, titleBlock, show = {}, texts = {}, comma = true, corridor,
}) {
  const [pageW, pageH] = PAPER_FORMATS[paperKey]
  const shown = { km: true, switches: true, trackNames: true, platforms: true, ...show }
  const reserve = titleBlock ? TITLE_COLUMN_MM : 0
  const area = drawingArea(pageW, pageH, reserve)
  const mmPerM = 1000 / scaleDen
  const kmOf = kmLines.length ? (track, point) => kmForTrackPoint(kmLines, track, point) : null
  const layout = schematicLayout({ tracks, switches, platforms, kmOf, leadTrackId, corridor })

  const furniture = (sheet) => {
    const items = []
    if (titleBlock) {
      items.push(...titleBlockFor(pageW, pageH, { ...titleBlock, legend: null }, sheet))
      const legend = [texts.legend, titleBlock.legend].filter(Boolean).join(' · ')
      const x = area.x + area.w + 3
      if (legend) {
        items.push(text(x, area.y + 5, [{ t: legend }], { size: fitSize(legend, STYLE.sizeLegend, reserve - 6) }))
      }
      // What the colours of the tracks and switches say.
      STATUSES.forEach((status, k) => {
        const label = texts.status?.[status]
        if (!label) return
        const y = area.y + 5 + (k + 1) * LEGEND_ROW
        items.push(line(x, y - 0.8, x + 8, y - 0.8, { width: STROKE.track, stroke: STATUS_COLOR[status] }))
        items.push(text(x + 10, y, [{ t: label }], { size: STYLE.sizeLegend }))
      })
    }
    items.push(...frameItems(pageW, pageH))
    return items
  }

  if (!layout) {
    const sheet = { index: 0, count: 1 }
    return { pageW, pageH, fits: true, sheets: [{ ...sheet, items: furniture(sheet) }] }
  }

  const [xa, xb] = layout.extent
  const span = (area.w - 2 * SIDE_PAD) / mmPerM
  const count = Math.max(1, Math.ceil((xb - xa) / span - 1e-9))
  const origin = (i) => (count === 1 ? (xa + xb) / 2 - span / 2 : xa + i * span)

  // Each sheet closes up the lanes it has nothing on, so neighbouring tracks
  // are always one gap apart; and every sheet uses the same gap, the widest
  // that lets the fullest of them fit.
  const reach = 2 * MAX_GAP / mmPerM
  const sheetLanes = Array.from({ length: count }, (_, i) => {
    const a = origin(i) - reach
    const b = origin(i) + span + reach
    const used = new Set()
    for (const st of layout.strands) {
      if (st.lane != null && !st.connector) {
        for (const sc of st.sections) if (sc.lane != null && sc.x1 >= a && sc.x0 <= b) used.add(sc.lane)
      }
      if (st.connector && st.x1 >= a && st.x0 <= b) {
        for (const n of st.endNodes) if (n?.lane != null) used.add(n.lane)
      }
    }
    return [...used].sort((p, q) => p - q)
  })
  const yTop = area.y + TOP_BAND
  const yBot = area.y + area.h - BOTTOM_BAND
  const most = Math.max(1, ...sheetLanes.map(l => l.length))
  const rawGap = most > 1 ? (yBot - yTop) / (most - 1) : MAX_GAP
  const gap = Math.min(MAX_GAP, rawGap)
  const yMid = (yTop + yBot) / 2
  const dxOf = (dyMm) => Math.abs(dyMm) / mmPerM
  const leg = Math.min(WEDGE_LEG, WEDGE_LEG * gap / MAX_GAP)

  /** Page y of a lane on a sheet that shows `lanes`, the highest lane on top. */
  const laneY = (lanes) => (lane) => {
    let rank = lanes.indexOf(lane)
    // A lane only touched from beyond the sheet sits between its neighbours.
    if (rank < 0) rank = lanes.filter(l => l < lane).length - 0.5
    return yMid - (rank - (lanes.length - 1) / 2) * gap
  }

  /** The strip for one lane spacing, in metres along x and page millimetres across (schematicStrip). */
  function stripFor(Y) {
    const { items } = schematicStrip(layout, {
      laneY: Y, dxOf, leg, gap, show: shown, sizes: { platformNear: PLATFORM_NEAR, platformFar: PLATFORM_FAR },
    })
    return items.map(it => {
      if (it.kind === 'line') return { ...it, width: it.connector ? STROKE.connector : STROKE.track, color: STATUS_COLOR[it.status] }
      if (it.kind === 'fill' || it.kind === 'dot') return { ...it, color: STATUS_COLOR[it.status] }
      if (it.kind === 'switch') {
        // In the sharp angle beside the triangle, as far out as the diagonal
        // leaves room for the figure's height.
        const off = Math.max(leg, SWITCH_CLEAR + CAP_HEIGHT * SWITCH_TEXT + 0.3)
        return { ...it, off, color: STATUS_COLOR[it.status] }
      }
      if (it.kind === 'label' && it.track) return { ...it, size: 1.8, prio: PRIO.track, room: it.room * mmPerM }
      if (it.kind === 'label') return { ...it, size: SWITCH_TEXT, color: STATUS_COLOR[it.status], prio: PRIO.switch }
      return it
    })
  }

  const stations = new Map()
  if (shown.platforms) {
    for (const pf of layout.platforms) {
      const name = String(pf.platform.stationName ?? '').trim()
      const code = String(pf.platform.code ?? '').trim().toUpperCase()
      const key = name.toLowerCase() || code
      if (!key) continue
      const st = stations.get(key) ?? { name, code, x0: pf.x0, x1: pf.x1 }
      st.x0 = Math.min(st.x0, pf.x0)
      st.x1 = Math.max(st.x1, pf.x1)
      if (!st.name && name) st.name = name
      if (!st.code && code) st.code = code
      stations.set(key, st)
    }
  }

  const marks = shown.km ? kmMarks(layout.kmTable, xa - span, xb + span) : []
  const posts = shown.km ? kmMarks(layout.kmTable, xa - span, xb + span, postStep(scaleDen)) : []

  const sheets = []
  for (let i = 0; i < count; i++) {
    const x0 = origin(i)
    const X = (x) => area.x + SIDE_PAD + (x - x0) * mmPerM
    const lanes = sheetLanes[i]
    const Y = laneY(lanes)
    const inner = []

    const yr = area.y + area.h - BOTTOM_BAND + 6
    for (const m of marks) {
      const px = X(m.x)
      const major = m.km % 1000 === 0
      const half = m.km % 500 === 0
      if (!major && !half && 100 * mmPerM < 2) continue
      inner.push(line(px, yr, px, yr - (major ? 3 : half ? 2 : 1.2), { width: STROKE.ruler }))
      if (major) {
        inner.push(text(px, yr + 3.5, [{ t: `km ${fmt(m.km / 1000, 1, comma)}` }],
          { size: 2.5, align: 'center', prio: PRIO.km }))
      }
    }
    if (marks.length) {
      inner.push(line(Math.max(area.x, X(xa)), yr, Math.min(area.x + area.w, X(xb)), yr, { width: STROKE.ruler }))
    }

    // The kilometre posts ride in the gap next to the reference, below it
    // where the sheet has a track there.
    if (posts.length && lanes.length) {
      const below = lanes.some(l => l < 0)
      const yp = Y(0) + (below || !lanes.some(l => l > 0) ? 1 : -1) * gap / 2
      for (const p of posts) {
        const px = X(p.x)
        inner.push(path(circlePath(px, yp, KM_POST_R), { width: STROKE.track, fill: '#ffffff' }))
        inner.push(text(px + KM_POST_TEXT, yp + 0.9, [{ t: fmt(p.km / 1000, 1, comma) }],
          { size: 2.47, bold: true, prio: PRIO.km }))
      }
    }

    for (const it of stripFor(Y)) {
      if (it.kind === 'line') {
        inner.push(path(poly(it.pts.map(([x, y]) => [X(x), y])), { width: it.width, stroke: it.color }))
      } else if (it.kind === 'fill') {
        inner.push(path([...poly(it.pts.map(([x, y]) => [X(x), y])), ['Z']], { stroke: null, fill: it.color }))
      } else if (it.kind === 'dot') {
        inner.push(path(circlePath(X(it.x), it.y, 0.55), { stroke: null, fill: it.color }))
      } else if (it.kind === 'platform') {
        const [a, b] = [X(it.x0), X(it.x1)]
        const edge = PLATFORM_EDGE * gap
        inner.push(path([['M', a, it.y0], ['L', b, it.y0], ['L', b, it.y1], ['L', a, it.y1], ['Z']],
          { width: STROKE.track, fill: '#ffffff' }))
        inner.push(line(a, it.y0 + edge, b, it.y0 + edge, { width: STROKE.platformEdge }))
        inner.push(line(a, it.y1 - edge, b, it.y1 - edge, { width: STROKE.platformEdge }))
      } else if (it.kind === 'label') {
        if (it.room != null && it.room < textWidth([{ t: it.t }], it.size) + 2) continue
        inner.push(text(X(it.x), it.y, [{ t: it.t }],
          { size: it.size, align: 'center', color: it.color, prio: it.prio }))
      } else if (it.kind === 'switch') {
        inner.push(switchLabel(X(it.x), it.y, it))
      }
    }

    for (const st of stations.values()) {
      const a = X(st.x0)
      const b = X(st.x1)
      const yb = area.y + 13
      inner.push(line(a, yb, b, yb, { width: STROKE.bracket }))
      inner.push(line(a, yb, a, yb + 1.5, { width: STROKE.bracket }))
      inner.push(line(b, yb, b, yb + 1.5, { width: STROKE.bracket }))
      const title = st.name || st.code
      inner.push(text((a + b) / 2, area.y + 7, [{ t: title }],
        { size: 3.0, align: 'center', bold: true, prio: PRIO.station }))
      if (st.name && st.code) {
        inner.push(text((a + b) / 2, area.y + 11, [{ t: st.code }], { size: 2.2, align: 'center', prio: PRIO.station }))
      }
    }

    if (i > 0 && texts.prev) {
      inner.push(text(area.x + 2, area.y + 4, [{ t: format(texts.prev, { n: i }) }], { size: 2.5, prio: PRIO.joint }))
    }
    if (i < count - 1 && texts.next) {
      inner.push(text(area.x + area.w - 2, area.y + 4, [{ t: format(texts.next, { n: i + 2 }) }],
        { size: 2.5, align: 'right', prio: PRIO.joint }))
    }

    const sheet = { index: i, count, ...sheetContext(layout, x0, x0 + span, titleBlock, texts, comma) }
    sheets.push({
      index: i, count,
      items: [{ type: 'group', clip: area, items: cullItems(inner, area) }, ...furniture(sheet)],
    })
  }

  return { pageW, pageH, sheets, fits: rawGap >= MIN_GAP, lanes: most, gap }
}
