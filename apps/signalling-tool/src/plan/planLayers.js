import { placeOf, stretchLine } from '../model/net.js'
import { labelOf } from '../model/domain.js'
import { signalTone, TONE_FILL } from './symbols.js'

/**
 * The plan as MapLibre layers, as the Layout-Tool draws its tracks: the map
 * draws them in the frame it draws the basemap in, so nothing trails behind
 * when it moves. Tracks, runs and stretches are lines, switches circles;
 * every object stands where it is in the plane — a foot on its track, a stem
 * out to its symbol at its offset on its side, the symbol an icon turned the
 * way it acts. Zoomed out, a symbol keeps a few pixels from its track: what
 * its offset lacks of them is added in pixels (icon-offset), evaluated per
 * whole zoom level as MapLibre lays symbols out. The icons and the names are
 * drawn here on a canvas, in the plan's colours.
 */

/** How far a symbol on a side stands from its track at the least [px]. */
export const SIDE_PX = 9
/** Pixels per metre at zoom 0 and the equator (Web Mercator, 512 px tiles). */
const PPM0 = 512 / 40075016.686
/** The whole zoom levels the pixels added to a symbol's offset are given for. */
const ZOOMS = Array.from({ length: 17 }, (_, i) => i + 6)
/** Where the label of a signal stands from its symbol, as the plan had it [px]. */
const LABEL_AT = [9, -7]
const LABEL_FONT_PX = 11
const LABEL_PAD = 3

const SRC = { tracks: 'plan-tracks', marked: 'plan-marked', switches: 'plan-switches', marks: 'plan-marks', preview: 'plan-preview' }

/** Pixels per metre at a latitude and zoom. */
export const pixelsPerMetre = (lat, zoom) => (PPM0 * 2 ** zoom) / Math.cos((lat * Math.PI) / 180)

/** The pixels a symbol is moved beyond its offset at a zoom, so it stands SIDE_PX apart at the least. */
export const extraPx = (offset, lat, zoom) => Math.max(0, SIDE_PX - offset * pixelsPerMetre(lat, zoom))

/**
 * An object to draw: where it is on its track, where its symbol stands in the
 * plane, and how it is turned. `side` is -1 left, 1 right of the stationing, 0
 * on the track; `along` 1 when it acts in the stationing's direction, else -1.
 */
export function placedMark(drawing, collection, obj, loc) {
  const p = placeOf(drawing, loc)
  if (!p) return null
  const on = drawing.frame.toLngLat(p.xy)
  const side = p.orphan ? 0 : loc.side === 'left' ? -1 : loc.side === 'right' ? 1 : 0
  const offset = side ? Math.max(loc.offset ?? 0, 0) : 0
  const b = ((p.bearing ?? p.angle ?? 0) * Math.PI) / 180
  // Right of the stationing in the frame (east, north): the bearing turned a quarter clockwise.
  const at = side ? drawing.frame.toLngLat([p.xy[0] + Math.cos(b) * offset * side, p.xy[1] - Math.sin(b) * offset * side]) : on
  const angle = p.angle ?? 0
  const along = Math.abs((((angle - (p.bearing ?? angle)) % 360) + 540) % 360 - 180) < 90 ? 1 : -1
  return { collection, obj, on, at, angle, bearing: b, side, along, offset, orphan: !!p.orphan }
}

/** Where a mark's symbol is on the screen of a map now: [x, y]. */
export function symbolOnScreen(map, mk) {
  const q = map.project(mk.at)
  if (!mk.side) return [q.x, q.y]
  const e = extraPx(mk.offset, mk.at[1], Math.floor(map.getZoom())) * mk.side
  return [q.x + Math.cos(mk.bearing) * e, q.y + Math.sin(mk.bearing) * e]
}

/** The marks of the domain: signals, detection points, balises and magnets, derailers. */
export function domainMarks(drawing, domain) {
  const out = []
  const put = (collection, obj, loc) => { const mk = placedMark(drawing, collection, obj, loc); if (mk) out.push(mk) }
  for (const s of domain.collections.signals) put('signals', s, s.at)
  for (const d of domain.collections.detectionPoints) put('detectionPoints', d, d.at)
  for (const b of domain.collections.balises) put('balises', b, b.at)
  for (const e of domain.collections.switchElements) if (e.derailer) put('switchElements', e, e.derailer.at)
  return out
}

/** The switches as dots, at the begin of their A port's track or its end. */
export function switchDots(drawing, net) {
  const out = []
  for (const sw of net.switches) {
    const a = sw.ports?.A
    const tr = a && drawing.tracks.get(a.trackId)
    if (!tr) continue
    const p = placeOf(drawing, { anchor: 'station', trackId: a.trackId, station: a.endpoint === 'BEGIN' ? 0 : tr.length })
    if (p) out.push({ id: sw.switchId, ll: drawing.frame.toLngLat(p.xy) })
  }
  return out
}

/** Which symbol an object is drawn with. */
function symbolOf(collection, obj) {
  if (collection === 'detectionPoints') return 'dp'
  if (collection === 'switchElements') return 'derailer'
  if (collection === 'balises') {
    if (obj.system === 'etcs') return 'etcs'
    return `pzb-${obj.system === 'pzb' ? String(obj.settings?.frequency ?? '').replace('+', '-') || 'other' : 'other'}`
  }
  return `sig-${signalTone(obj)}`
}

const feature = (geometry, properties) => ({ type: 'Feature', geometry, properties })
const point = (ll) => ({ type: 'Point', coordinates: ll })
const line = (lls) => ({ type: 'LineString', coordinates: lls })
const collection = (features) => ({ type: 'FeatureCollection', features })

/** The features of marks: a stem and a foot where a symbol stands beside its track, the symbol, a signal's name. */
function markFeatures(marks, { isSelected, names, t }) {
  const out = []
  for (const mk of marks) {
    const sel = isSelected(mk)
    // The pixels added zoomed out: along the icon's x when it is turned, on the screen for the label.
    const extra = Object.fromEntries(ZOOMS.map(z => [z, extraPx(mk.offset, mk.at[1], z) * mk.side]))
    const props = { kind: 'mark', collection: mk.collection, id: mk.obj.id, rotate: mk.angle, orphan: mk.orphan, icon: `ra-${symbolOf(mk.collection, mk.obj)}${sel ? '-sel' : ''}` }
    for (const z of ZOOMS) {
      props[`i${z}`] = [extra[z] * mk.along, 0]
      props[`s${z}`] = [(extra[z] - (SIDE_PX / 2) * mk.side) * mk.along, 0]
    }
    if (mk.side) {
      out.push(feature(line([mk.on, mk.at]), { kind: 'stem' }))
      out.push(feature(point(mk.on), { kind: 'foot' }))
      out.push(feature(point(mk.at), { ...props, kind: 'stub' }))
    }
    out.push(feature(point(mk.at), props))
    const name = names && mk.collection === 'signals' && mk.obj.name ? labelOf('signals', mk.obj, t) : null
    if (name) {
      const label = { kind: 'label', icon: `ra-label:${name}` }
      for (const z of ZOOMS) {
        label[`l${z}`] = [LABEL_AT[0] - LABEL_PAD + Math.cos(mk.bearing) * extra[z], LABEL_AT[1] + LABEL_PAD + Math.sin(mk.bearing) * extra[z]]
      }
      out.push(feature(point(mk.at), label))
    }
  }
  return out
}

/**
 * The plan's data, one FeatureCollection per source: { tracks, marked,
 * switches, marks, preview }, and the labels' texts whose icons it needs.
 */
export function planData({ drawing, marks, switches, selection, highlight, preview, snap, t }) {
  const isSel = (collection, id) => selection?.collection === collection && selection.id === id
  const lineOf = (pts) => pts.map(p => drawing.frame.toLngLat(p))
  const tracks = [...drawing.tracks.values()].map(tr => feature(line(lineOf(tr.line)), { id: tr.id }))
  const marked = [
    ...(highlight?.runs ?? []).flatMap(id => { const tr = drawing.tracks.get(id); return tr ? [feature(line(lineOf(tr.line)), { kind: 'run' })] : [] }),
    ...(highlight?.stretches ?? []).map(s => feature(line(lineOf(stretchLine(drawing, s.trackId, s.from, s.to))), { kind: 'stretch', tone: s.tone ?? '' })),
  ].filter(f => f.geometry.coordinates.length > 1)
  const shown = marks.filter(mk => !(preview && mk.obj.id === preview.obj.id))
  const markSel = (mk) => isSel(mk.collection, mk.obj.id) || !!highlight?.ids?.includes(mk.obj.id)
  const data = {
    tracks: collection(tracks),
    marked: collection(marked),
    switches: collection(switches.map(s => feature(point(s.ll), { id: s.id, sel: isSel('switches', s.id) }))),
    marks: collection(markFeatures(shown, { isSelected: markSel, names: true, t })),
    preview: collection([
      ...(snap ? [feature(point(snap), { kind: 'snap' })] : []),
      ...(preview ? markFeatures([preview], { isSelected: () => true, names: false, t }) : []),
    ]),
  }
  const labels = data.marks.features.filter(f => f.properties.kind === 'label').map(f => f.properties.icon)
  return { data, labels }
}

/** The plan's colours, as the stylesheet states them for the plan's box. */
export function paletteOf(el) {
  const css = getComputedStyle(el)
  const v = (name, fallback) => css.getPropertyValue(name).trim() || fallback
  return {
    track: v('--plan-track', '#2a2d3a'), mark: v('--plan-mark', '#1a1a2e'), primary: v('--color-primary', '#303383'),
    bg: v('--ui-bg', '#ffffff'), surface: v('--ui-surface', '#f6f7fb'), font: v('--ui-font', 'system-ui, sans-serif'),
  }
}

// ── the icons ───────────────────────────────────────────────────────────

const ICON_PX = 32
const RATIO = 2
const DP_BLUE = '#1f6fb5'
const BALISE = { 'pzb-500': '#d0308f', 'pzb-1000': '#7b3fbf', 'pzb-2000': '#0f9fb5', 'pzb-1000-2000': '#0f9fb5', 'pzb-other': '#7b3fbf' }
const SYMBOLS = ['sig-main', 'sig-distant', 'sig-shunting', 'sig-board', 'sig-marker', 'sig-virtual', 'dp', 'etcs', ...Object.keys(BALISE), 'derailer']

/** A canvas of the icon's size, its origin in the middle and +x the way the object acts — up in the icon, as icon-rotate turns it. */
function iconCanvas(size = ICON_PX) {
  const c = document.createElement('canvas')
  c.width = c.height = size * RATIO
  const g = c.getContext('2d')
  g.scale(RATIO, RATIO)
  g.translate(size / 2, size / 2)
  g.rotate(-Math.PI / 2)
  g.lineCap = 'butt'
  return { c, g }
}

const imageOf = ({ c, g }) => g.getImageData(0, 0, c.width, c.height)

/** One symbol, as the plan drew it in SVG: a signal a short mast with its head ahead, a board a square … */
function drawSymbol(name, sel, pal) {
  const cv = iconCanvas()
  const { g } = cv
  const stroke = (path, color, width, dash) => { g.strokeStyle = color; g.lineWidth = width; g.setLineDash(dash ?? []); g.stroke(new Path2D(path)) }
  const fill = (path, color) => { if (color && color !== 'none') { g.fillStyle = color; g.fill(new Path2D(path)) } }
  const edge = sel ? pal.primary : pal.mark
  const edgeWidth = (w) => (sel ? 3 : w)
  if (name === 'dp') {
    stroke('M0,-6 L0,6', sel ? pal.primary : DP_BLUE, edgeWidth(2))
    fill('M2.2,0 A2.2,2.2 0 1 0 -2.2,0 A2.2,2.2 0 1 0 2.2,0 Z', DP_BLUE)
  } else if (name === 'derailer') {
    stroke('M-4,-4 L4,4 M-4,4 L4,-4', '#c0392b', 2)
  } else if (name === 'etcs') {
    const d = 'M0,-4 L4,0 L0,4 L-4,0 Z'
    fill(d, '#f2c200')
    stroke(d, sel ? pal.primary : '#333', sel ? 3 : 1)
  } else if (name.startsWith('pzb-')) {
    const d = 'M-3,-3 L3,-3 L3,3 L-3,3 Z'
    fill(d, BALISE[name])
    if (sel) stroke(d, pal.primary, 3)
    else if (name === 'pzb-1000-2000') stroke(d, '#7b3fbf', 1.5)
  } else {
    const tone = name.slice(4)
    if (tone === 'virtual') {
      stroke('M4,0 A4,4 0 1 0 -4,0 A4,4 0 1 0 4,0 Z', edge, edgeWidth(1.2), [2, 2])
    } else if (tone === 'board' || tone === 'marker') {
      const d = 'M-3.5,-3.5 L3.5,-3.5 L3.5,3.5 L-3.5,3.5 Z'
      fill(d, TONE_FILL[tone])
      stroke(d, edge, edgeWidth(1.2))
    } else {
      stroke('M-6,0 L2,0', edge, edgeWidth(1.6))
      const head = 'M10,0 A4,4 0 1 0 2,0 A4,4 0 1 0 10,0 Z'
      fill(head, TONE_FILL[tone])
      stroke(head, edge, edgeWidth(1.2))
    }
  }
  return imageOf(cv)
}

/** The last stretch of a stem, SIDE_PX long across the icon's x: from the track to a symbol pushed apart. */
function drawStub(pal) {
  const cv = iconCanvas(SIDE_PX * 2 + 2)
  cv.g.rotate(Math.PI / 2)
  cv.g.globalAlpha = 0.6
  cv.g.strokeStyle = pal.track
  cv.g.lineWidth = 1
  cv.g.beginPath(); cv.g.moveTo(-SIDE_PX / 2, 0); cv.g.lineTo(SIDE_PX / 2, 0); cv.g.stroke()
  return imageOf(cv)
}

/** A signal's name, with a halo of the surface's colour; its baseline LABEL_PAD above the bottom. */
function drawLabel(text, pal) {
  const font = `600 ${LABEL_FONT_PX}px ${pal.font}`
  const probe = document.createElement('canvas').getContext('2d')
  probe.font = font
  const w = Math.ceil(probe.measureText(text).width) + LABEL_PAD * 2
  const h = LABEL_FONT_PX + LABEL_PAD * 2
  const c = document.createElement('canvas')
  c.width = w * RATIO; c.height = h * RATIO
  const g = c.getContext('2d')
  g.scale(RATIO, RATIO)
  g.font = font
  g.lineJoin = 'round'
  g.strokeStyle = pal.surface; g.lineWidth = 3
  g.strokeText(text, LABEL_PAD, h - LABEL_PAD)
  g.fillStyle = pal.mark
  g.fillText(text, LABEL_PAD, h - LABEL_PAD)
  return g.getImageData(0, 0, c.width, c.height)
}

/** The icons the plan needs on a map, drawn in its colours; all drawn anew when the colours changed. */
export function ensureImages(map, pal, labels) {
  const key = JSON.stringify(pal)
  if (map.__planPalette !== key) {
    for (const id of map.listImages()) if (id.startsWith('ra-')) map.removeImage(id)
    map.__planPalette = key
  }
  const add = (id, draw) => { if (!map.hasImage(id)) map.addImage(id, draw(), { pixelRatio: RATIO }) }
  for (const s of SYMBOLS) {
    add(`ra-${s}`, () => drawSymbol(s, false, pal))
    add(`ra-${s}-sel`, () => drawSymbol(s, true, pal))
  }
  add('ra-stub', () => drawStub(pal))
  for (const id of labels) add(id, () => drawLabel(id.slice('ra-label:'.length), pal))
}

// ── the layers ──────────────────────────────────────────────────────────

/** A pixel offset given per whole zoom level in the features' properties `<prefix><zoom>`. */
const offsetBy = (prefix) => ['interpolate', ['linear'], ['zoom'], ...ZOOMS.flatMap(z => [z, ['array', 'number', 2, ['get', `${prefix}${z}`]]])]
const kind = (k) => ['==', ['get', 'kind'], k]
const SYMBOL = { 'icon-allow-overlap': true, 'icon-ignore-placement': true, 'icon-rotation-alignment': 'map', 'icon-pitch-alignment': 'map' }

/** The layers of marks from one source: stems, feet, stubs, symbols, labels. */
function markLayers(prefix, source, pal, opacity) {
  return [
    { id: `${prefix}-stems`, type: 'line', source, filter: kind('stem'), paint: { 'line-color': pal.track, 'line-width': 1, 'line-opacity': 0.6 * opacity } },
    { id: `${prefix}-feet`, type: 'circle', source, filter: kind('foot'), paint: { 'circle-color': pal.track, 'circle-radius': 1.6, 'circle-opacity': opacity } },
    { id: `${prefix}-stubs`, type: 'symbol', source, filter: kind('stub'),
      layout: { ...SYMBOL, 'icon-image': 'ra-stub', 'icon-rotate': ['get', 'rotate'], 'icon-offset': offsetBy('s') }, paint: { 'icon-opacity': opacity } },
    { id: `${prefix}-symbols`, type: 'symbol', source, filter: kind('mark'),
      layout: { ...SYMBOL, 'icon-image': ['get', 'icon'], 'icon-rotate': ['get', 'rotate'], 'icon-offset': offsetBy('i') },
      paint: { 'icon-opacity': ['case', ['get', 'orphan'], 0.6 * opacity, opacity] } },
    { id: `${prefix}-labels`, type: 'symbol', source, filter: kind('label'),
      layout: { ...SYMBOL, 'icon-image': ['get', 'icon'], 'icon-anchor': 'bottom-left', 'icon-rotation-alignment': 'viewport', 'icon-offset': offsetBy('l') },
      paint: { 'icon-opacity': opacity } },
  ]
}

/**
 * The plan's sources and layers on a map whose style has loaded — again after
 * every new basemap, which takes them away. `nameZoom` is the zoom from which
 * the signals' names are shown.
 */
export function addPlanLayers(map, pal, data, nameZoom) {
  if (map.getSource(SRC.tracks)) return
  for (const [k, id] of Object.entries(SRC)) map.addSource(id, { type: 'geojson', data: data?.[k] ?? collection([]), maxzoom: 22 })
  const round = { 'line-cap': 'round', 'line-join': 'round' }
  const layers = [
    // On a basemap a track keeps a light edge, so it reads on any ground.
    { id: 'plan-track-edges', type: 'line', source: SRC.tracks, layout: round, paint: { 'line-color': '#ffffff', 'line-width': 4.8, 'line-opacity': 0.9, 'line-blur': 1.2 } },
    { id: 'plan-tracks', type: 'line', source: SRC.tracks, layout: round, paint: { 'line-color': pal.track, 'line-width': 2.4 } },
    { id: 'plan-runs', type: 'line', source: SRC.marked, filter: kind('run'), layout: round, paint: { 'line-color': pal.primary, 'line-width': 6, 'line-opacity': 0.45 } },
    { id: 'plan-stretches', type: 'line', source: SRC.marked, filter: kind('stretch'),
      paint: { 'line-color': ['case', ['==', ['get', 'tone'], 'warn'], '#d68910', '#1f9e3a'], 'line-width': 7, 'line-opacity': 0.5 } },
    { id: 'plan-switches', type: 'circle', source: SRC.switches,
      paint: { 'circle-radius': ['case', ['get', 'sel'], 5, 3], 'circle-color': ['case', ['get', 'sel'], pal.primary, pal.bg],
        'circle-stroke-color': ['case', ['get', 'sel'], pal.primary, pal.track], 'circle-stroke-width': 1.5 } },
    ...markLayers('plan-marks', SRC.marks, pal, 1),
    ...markLayers('plan-preview', SRC.preview, pal, 0.75),
    { id: 'plan-snap', type: 'circle', source: SRC.preview, filter: kind('snap'),
      paint: { 'circle-radius': 4, 'circle-color': 'rgba(0,0,0,0)', 'circle-stroke-color': pal.primary, 'circle-stroke-width': 2 } },
  ]
  for (const layer of layers) map.addLayer(layer)
  map.setLayerZoomRange('plan-marks-labels', nameZoom, 24)
}

/** The plan's data on its sources, the names' zoom on their layer, the colours on the layers' paint. */
export function setPlanData(map, pal, data, nameZoom) {
  if (!map.getSource(SRC.tracks)) return
  for (const [k, id] of Object.entries(SRC)) map.getSource(id).setData(data[k])
  map.setLayerZoomRange('plan-marks-labels', nameZoom, 24)
  map.setPaintProperty('plan-tracks', 'line-color', pal.track)
  map.setPaintProperty('plan-runs', 'line-color', pal.primary)
  map.setPaintProperty('plan-snap', 'circle-stroke-color', pal.primary)
  map.setPaintProperty('plan-switches', 'circle-color', ['case', ['get', 'sel'], pal.primary, pal.bg])
  map.setPaintProperty('plan-switches', 'circle-stroke-color', ['case', ['get', 'sel'], pal.primary, pal.track])
  for (const p of ['plan-marks', 'plan-preview']) {
    map.setPaintProperty(`${p}-stems`, 'line-color', pal.track)
    map.setPaintProperty(`${p}-feet`, 'circle-color', pal.track)
  }
}

/** The zoom from which names are shown at a latitude: where a metre is more than 0.6 px, as the plan had it. */
export const nameZoomAt = (lat) => Math.min(24, Math.max(0, Math.log2((0.6 * Math.cos((lat * Math.PI) / 180)) / PPM0)))
