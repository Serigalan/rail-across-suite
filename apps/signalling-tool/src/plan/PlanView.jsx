import { useEffect, useMemo, useRef, useState } from 'react'
import { findBasemap, isElevationBasemap, updateElevationRange } from '@rail-across/basemaps'
import { locate, placeOf } from '../model/net.js'
import {
  addPlanLayers, domainMarks, ensureImages, nameZoomAt, paletteOf, pixelsPerMetre, placedMark, placeLines, planData, setPlanData, switchDots, symbolOnScreen,
} from './planLayers.js'

/**
 * The plan: the net in its true position on the basemap chosen in the layers
 * panel (the suite's basemaps, as the Layout-Tool has them), every track a
 * line, every switch a dot, and on it what the domain holds — signals by
 * their kind, boards, detection points, balises and magnets. As OSRD's map
 * has them, each stands where it is in the plane: a dot on its track at its
 * station, its symbol beside it at its offset on its side, turned the way it
 * acts — never closer than a few pixels, so it stays apart from the track
 * when zoomed out. A stopping place is a band under its track, as a platform
 * lies beside it. What is chosen is drawn in the primary colour, with its
 * run, its section or its overlap; ways found, the one shown with arrows the
 * way it runs. The map moves and zooms (MapLibre, north
 * up) and draws the plan as its own layers (planLayers.js), in the frame it
 * draws the basemap in; a symbol keeps its size in pixels, so it stays
 * readable at every zoom. A click picks what is nearest. While placing, as in OSRD's
 * editor, the mouse snaps to the nearest track within reach and the object is
 * drawn there before the click sets it. Picking a stretch — a stopping place
 * where the signals bound it, or its two ends —, the mouse snaps to a track
 * the same way and what the click would take is drawn dashed; picking a
 * stopping place takes the one clicked.
 */

const PICK_PX = 12
/** The style without a map: the plan on the page's own ground. */
const PLAIN = { version: 8, sources: {}, layers: [{ id: 'ground', type: 'background', paint: { 'background-color': 'rgba(0,0,0,0)' } }] }
export const NO_BASEMAP = 'none'

const styleOf = (basemapId) => (basemapId === NO_BASEMAP ? PLAIN : findBasemap(basemapId)?.style ?? PLAIN)

export default function PlanView({
  hidden, drawing, net, domain, selection, highlight, mode, onPick, onPlace, onPickPoint, onPickStretch, onPickPlace, fitKey, focus, basemap, t,
}) {
  const box = useRef(null)
  const mapBox = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const [hover, setHover] = useState(null)   // where placing or picking a stretch would set, snapped to a track
  const [scheme, setScheme] = useState(0)   // a new one whenever light and dark change, for the plan's colours
  const handlers = useRef({})
  const plan = useRef(null)   // what the map's layers show: { pal, data, labels, nameZoom }
  const styled = useRef(false)   // the map's style has loaded: the plan's layers can go on it

  // The map, once: north up, no tilt; the plan's layers with every style it loads.
  useEffect(() => {
    let gone = false
    import('./maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !mapBox.current) return
      const m = new maplibregl.Map({
        container: mapBox.current, style: styleOf(basemap), attributionControl: { compact: true },
        dragRotate: false, pitchWithRotate: false, touchPitch: false, maxPitch: 0, center: [10, 51], zoom: 5,
      })
      m.touchZoomRotate.disableRotation()
      m.keyboard.disableRotation()
      m.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left')
      // A style given as an object may have loaded before the listener was there: `load` follows it.
      const onStyle = () => {
        styled.current = true
        const p = plan.current
        if (!p) return
        ensureImages(m, p.pal, p.labels)
        addPlanLayers(m, p.pal, p.data, p.nameZoom)
      }
      m.on('style.load', onStyle)
      m.on('load', onStyle)
      m.on('click', (e) => handlers.current.click?.(e.point.x, e.point.y))
      m.on('mousemove', (e) => handlers.current.move?.(e.point.x, e.point.y))
      m.on('mouseout', () => handlers.current.move?.(null))
      m.on('moveend', () => { if (isElevationBasemap(handlers.current.basemap)) updateElevationRange(m) })
      map.current = m
      setReady(true)
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
    // The map is made once; the basemap changes through setStyle below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Light or dark: the plan's colours follow the page's.
  useEffect(() => {
    const again = () => setScheme(n => n + 1)
    const dark = window.matchMedia?.('(prefers-color-scheme: dark)')
    dark?.addEventListener('change', again)
    const watch = new MutationObserver(again)
    watch.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
    return () => { dark?.removeEventListener('change', again); watch.disconnect() }
  }, [])

  // The basemap chosen.
  const shown = useRef(null)
  useEffect(() => {
    const m = map.current
    if (!ready || !m || shown.current === basemap) return
    if (shown.current !== null) { styled.current = false; m.setStyle(styleOf(basemap)) }
    shown.current = basemap
    handlers.current.basemap = basemap
    if (isElevationBasemap(basemap)) m.once('idle', () => updateElevationRange(m, { force: true }))
  }, [ready, basemap])

  // The whole net in view: when it is first there, and again on asking — once
  // the plan is shown, as a hidden map has no size to fit it into.
  const fitted = useRef(null)
  useEffect(() => {
    const m = map.current
    if (!ready || !m || hidden) return
    if (fitted.current?.drawing === drawing && fitted.current?.fitKey === fitKey) return
    fitted.current = { drawing, fitKey }
    const [x0, y0, x1, y1] = drawing.bounds
    const sw = drawing.frame.toLngLat([x0, y0]), ne = drawing.frame.toLngLat([x1, y1])
    m.resize()
    m.fitBounds([sw, ne], { padding: 40, animate: false, maxZoom: 19 })
  }, [ready, drawing, fitKey, hidden])

  // An object chosen in a list is brought into view, close enough to tell it
  // from its neighbours; what reaches further — a way — as a whole.
  useEffect(() => {
    const m = map.current
    if (!ready || !m || !focus) return
    if (focus.fit) {
      const lls = focus.fit.map(loc => placeOf(drawing, loc)).filter(Boolean).map(p => drawing.frame.toLngLat(p.xy))
      if (!lls.length) return
      const lng = lls.map(l => l[0]), lat = lls.map(l => l[1])
      m.fitBounds([[Math.min(...lng), Math.min(...lat)], [Math.max(...lng), Math.max(...lat)]], { padding: { top: 90, bottom: 70, left: 60, right: 60 }, duration: 400, maxZoom: 17 })
      return
    }
    const p = placeOf(drawing, focus.at)
    if (!p) return
    m.easeTo({ center: drawing.frame.toLngLat(p.xy), zoom: Math.max(m.getZoom(), 16.5), duration: 400 })
  }, [ready, focus, drawing])

  // The plan on the map: its sources and layers, again after every new basemap.
  const marks = useMemo(() => domainMarks(drawing, domain), [domain, drawing])
  const switches = useMemo(() => switchDots(drawing, net), [net, drawing])
  const places = useMemo(() => placeLines(drawing, domain.collections.stoppingPlaces), [domain, drawing])

  const m = ready ? map.current : null
  /** The mark of each object's symbol nearest a point of the screen, within reach. */
  const pickAt = (px, py) => {
    let best = null
    for (const mk of marks) {
      const [sx, sy] = symbolOnScreen(m, mk)
      const d = Math.hypot(sx - px, sy - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: mk.collection, id: mk.obj.id }
    }
    if (best) return best
    for (const s of switches) {
      const q = m.project(s.ll)
      const d = Math.hypot(q.x - px, q.y - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: 'switches', id: s.id }
    }
    return best ?? placeAt(px, py)
  }
  /** The stopping place whose band is nearest a point of the screen, within reach. */
  const placeAt = (px, py) => {
    let best = null
    for (const h of places) {
      const pts = h.line.map(ll => m.project(ll))
      for (let i = 1; i < pts.length; i++) {
        const d = toSegment(px, py, pts[i - 1], pts[i])
        if (d < PICK_PX / 2 + 3 && (!best || d < best.d)) best = { d, collection: 'stoppingPlaces', id: h.id }
      }
    }
    return best
  }

  /** The place on the nearest track within reach of a point of the screen, or null. */
  const snapAt = (px, py) => {
    const ll = m.unproject([px, py])
    return locate(drawing, drawing.frame.toLocal([ll.lng, ll.lat]), (PICK_PX * 1.6) / pixelsPerMetre(ll.lat, m.getZoom()) + 2)
  }
  /** Where a pick of a stretch is: on a track, with how far it reaches there [m]. */
  const stretchHit = (px, py) => {
    const hit = snapAt(px, py)
    return hit && { ...hit, reach: PICK_PX / pixelsPerMetre(m.unproject([px, py]).lat, m.getZoom()) }
  }
  const snaps = mode?.kind === 'place' || mode?.kind === 'pick-stretch'
  // Placing: the mouse snaps to a track, and what would be set is drawn there.
  handlers.current.move = (px, py) => {
    if (!m || !snaps) return
    const hit = px == null ? null : mode.kind === 'place' ? snapAt(px, py) : stretchHit(px, py)
    setHover(h => (h?.trackId === hit?.trackId && h?.station === hit?.station && h?.side === hit?.side ? h : hit))
  }
  // What a click does, as it is now (the map's listener is made once).
  handlers.current.click = (px, py) => {
    if (!m) return
    if (mode?.kind === 'place') {
      const hit = snapAt(px, py)
      if (hit && mode.make(hit)) { setHover(null); onPlace?.(hit) }
      return
    }
    if (mode?.kind === 'pick-stretch') {
      const stretch = mode.stretchAt(stretchHit(px, py))
      if (stretch) onPickStretch?.(stretch)
      return
    }
    if (mode?.kind === 'pick-place') {
      const place = placeAt(px, py)
      if (place) onPickPlace?.(place.id)
      return
    }
    const hit = pickAt(px, py)
    if (mode?.kind === 'pick-points') {
      if (hit?.collection === 'detectionPoints') onPickPoint?.(hit.id)
      return
    }
    onPick?.(hit ? { collection: hit.collection, id: hit.id } : null)
  }
  // What a pick of a stretch would take where the mouse is.
  const stretch = useMemo(() => (mode?.kind === 'pick-stretch' && hover ? mode.stretchAt(hover) : null), [mode, hover])
  useEffect(() => {
    if (!m) return
    const refused = (mode?.kind === 'place' && !(hover && mode.make(hover))) || (mode?.kind === 'pick-stretch' && !stretch)
    m.getCanvas().style.cursor = mode ? (refused ? 'not-allowed' : 'crosshair') : ''
  }, [m, mode, hover, stretch])
  useEffect(() => { if (!snaps) setHover(null) }, [snaps])

  // What placing would set where the mouse is, drawn as the plan draws it.
  const preview = useMemo(() => {
    if (mode?.kind !== 'place' || !hover) return null
    const obj = mode.make(hover)
    return obj ? placedMark(drawing, mode.collection, obj, locOf(mode.collection, obj)) : null
  }, [mode, hover, drawing])

  // The data on the map's sources whenever it changes; the map draws it as it moves.
  useEffect(() => {
    if (!m || !box.current) return
    const pal = paletteOf(box.current)
    const snap = preview?.on ?? (stretch && hover ? drawing.frame.toLngLat(placeOf(drawing, hover).xy) : null)
    const { data, labels } = planData({ drawing, places, marks, switches, selection, highlight, preview, stretch, snap, t })
    const nameZoom = nameZoomAt(drawing.frame.toLngLat([0, 0])[1])
    plan.current = { pal, data, labels, nameZoom }
    if (!styled.current) return   // style.load puts them on
    ensureImages(m, pal, labels)
    if (m.getSource('plan-tracks')) setPlanData(m, pal, data, nameZoom)
    else addPlanLayers(m, pal, data, nameZoom)
  }, [m, drawing, places, marks, switches, selection, highlight, preview, stretch, hover, t, scheme])

  return (
    <div ref={box} className={`plan${hidden ? ' is-hidden' : ''}`}>
      <div className="plan-map"><div ref={mapBox} className="plan-map-inner" /></div>
    </div>
  )
}

/** Distance of a point of the screen from a segment of it [px]. */
function toSegment(px, py, a, b) {
  const dx = b.x - a.x, dy = b.y - a.y
  const len2 = dx * dx + dy * dy
  const k = len2 ? Math.max(0, Math.min(1, ((px - a.x) * dx + (py - a.y) * dy) / len2)) : 0
  return Math.hypot(px - (a.x + k * dx), py - (a.y + k * dy))
}

/** Where an object of a collection stands: its location, a derailer's its own. */
const locOf = (collection, obj) => (collection === 'switchElements' ? obj.derailer?.at : obj.at)
