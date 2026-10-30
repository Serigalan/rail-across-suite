import { useEffect, useMemo, useRef, useState } from 'react'
import { findBasemap, isElevationBasemap, updateElevationRange } from '@rail-across/basemaps'
import { locate, placeOf, stretchLine } from '../model/net.js'
import { runStretches } from '../model/runs.js'
import { signalTone, TONE_FILL } from './symbols.js'
import { labelOf } from '../model/domain.js'

/**
 * The plan: the net in its true position on the basemap chosen in the layers
 * panel (the suite's basemaps, as the Layout-Tool has them), every track a
 * line, every switch a dot, and on it what the domain holds — signals by
 * their kind, boards, detection points, balises and magnets. As OSRD's map
 * has them, each stands where it is in the plane: a dot on its track at its
 * station, its symbol beside it at its offset on its side, turned the way it
 * acts — never closer than a few pixels, so it stays apart from the track
 * when zoomed out. What is chosen is drawn in the primary colour, with its
 * run, its section or its overlap. The map moves and zooms (MapLibre, north
 * up); the plan is drawn over it in screen pixels, so a symbol stays readable
 * at every zoom. A click picks what is nearest. While placing, as in OSRD's
 * editor, the mouse snaps to the nearest track within reach and the object is
 * drawn there before the click sets it.
 */

const PICK_PX = 12
/** How far a symbol on a side stands from its track at the least [px]. */
const SIDE_PX = 9
/** The style without a map: the plan on the page's own ground. */
const PLAIN = { version: 8, sources: {}, layers: [{ id: 'ground', type: 'background', paint: { 'background-color': 'rgba(0,0,0,0)' } }] }
export const NO_BASEMAP = 'none'

const styleOf = (basemapId) => (basemapId === NO_BASEMAP ? PLAIN : findBasemap(basemapId)?.style ?? PLAIN)

export default function PlanView({ drawing, net, domain, selection, highlight, mode, onPick, onPlace, onPickPoint, fitKey, focus, basemap, t }) {
  const box = useRef(null)
  const mapBox = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const [, setFrame] = useState(0)   // a new frame whenever the map moved
  const [hover, setHover] = useState(null)   // where placing would set, snapped to a track
  const handlers = useRef({})

  // The map, once: north up, no tilt; every move draws the plan anew (once a frame).
  useEffect(() => {
    let gone = false
    let raf = 0
    import('./maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !mapBox.current) return
      const m = new maplibregl.Map({
        container: mapBox.current, style: styleOf(basemap), attributionControl: { compact: true },
        dragRotate: false, pitchWithRotate: false, touchPitch: false, maxPitch: 0, center: [10, 51], zoom: 5,
      })
      m.touchZoomRotate.disableRotation()
      m.keyboard.disableRotation()
      m.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left')
      const redraw = () => { if (!raf) raf = requestAnimationFrame(() => { raf = 0; setFrame(f => f + 1) }) }
      m.on('move', redraw)
      m.on('resize', redraw)
      m.on('click', (e) => handlers.current.click?.(e.point.x, e.point.y))
      m.on('mousemove', (e) => handlers.current.move?.(e.point.x, e.point.y))
      m.on('mouseout', () => handlers.current.move?.(null))
      m.on('moveend', () => { if (isElevationBasemap(handlers.current.basemap)) updateElevationRange(m) })
      map.current = m
      setReady(true)
    })
    return () => { gone = true; if (raf) cancelAnimationFrame(raf); map.current?.remove(); map.current = null }
    // The map is made once; the basemap changes through setStyle below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // The basemap chosen.
  const shown = useRef(null)
  useEffect(() => {
    const m = map.current
    if (!ready || !m || shown.current === basemap) return
    if (shown.current !== null) m.setStyle(styleOf(basemap))
    shown.current = basemap
    handlers.current.basemap = basemap
    if (isElevationBasemap(basemap)) m.once('idle', () => updateElevationRange(m, { force: true }))
  }, [ready, basemap])

  // The whole net in view: when it is first there, and again on asking.
  useEffect(() => {
    const m = map.current
    if (!ready || !m) return
    const [x0, y0, x1, y1] = drawing.bounds
    const sw = drawing.frame.toLngLat([x0, y0]), ne = drawing.frame.toLngLat([x1, y1])
    m.fitBounds([sw, ne], { padding: 40, animate: false, maxZoom: 19 })
  }, [ready, drawing, fitKey])

  // An object chosen in a list is brought into view, close enough to tell it from its neighbours.
  useEffect(() => {
    const m = map.current
    if (!ready || !m || !focus) return
    const p = placeOf(drawing, focus.at)
    if (!p) return
    m.easeTo({ center: drawing.frame.toLngLat(p.xy), zoom: Math.max(m.getZoom(), 16.5), duration: 400 })
  }, [ready, focus, drawing])

  // The frame's points in WGS84, once per drawing; projected on every frame.
  const lines = useMemo(() => [...drawing.tracks.values()].map(tr => ({ id: tr.id, ll: tr.line.map(p => drawing.frame.toLngLat(p)) })), [drawing])
  const marks = useMemo(() => {
    const out = []
    const put = (collection, obj, loc) => {
      const p = placeOf(drawing, loc)
      if (p) out.push({ collection, obj, p, ll: drawing.frame.toLngLat(p.xy) })
    }
    for (const s of domain.collections.signals) put('signals', s, s.at)
    for (const d of domain.collections.detectionPoints) put('detectionPoints', d, d.at)
    for (const b of domain.collections.balises) put('balises', b, b.at)
    for (const e of domain.collections.switchElements) if (e.derailer) put('switchElements', e, e.derailer.at)
    for (const v of domain.collections.speedSections) put('speedSections', v, v.start)
    return out
  }, [domain, drawing])
  const switchDots = useMemo(() => {
    const out = []
    for (const sw of net.switches) {
      const a = sw.ports?.A
      const tr = a && drawing.tracks.get(a.trackId)
      if (!tr) continue
      const p = placeOf(drawing, { anchor: 'station', trackId: a.trackId, station: a.endpoint === 'BEGIN' ? 0 : tr.length })
      if (p) out.push({ id: sw.switchId, ll: drawing.frame.toLngLat(p.xy) })
    }
    return out
  }, [net, drawing])

  const m = ready ? map.current : null
  const project = (ll) => { const q = m.project(ll); return [q.x, q.y] }
  // Pixels per metre at the middle of the view (Web Mercator, 512 px tiles).
  const scale = m ? (512 * 2 ** m.getZoom()) / (40075016.686 * Math.cos((m.getCenter().lat * Math.PI) / 180)) : 1

  /** A mark on screen: `on` the point of its track, `at` its symbol — its offset to its side, a few pixels at the least. */
  const screenOf = (mk) => {
    const on = project(mk.ll)
    const loc = locOf(mk.collection, mk.obj)
    const sx = loc?.side === 'left' ? -1 : loc?.side === 'right' ? 1 : 0
    if (!sx || mk.p.orphan) return { on, at: on }
    const px = Math.max((loc.offset ?? 0) * scale, SIDE_PX)
    // Right of the stationing: the bearing turned a quarter clockwise; on screen y runs down.
    const b = ((mk.p.bearing ?? mk.p.angle) * Math.PI) / 180
    return { on, at: [on[0] + Math.cos(b) * px * sx, on[1] + Math.sin(b) * px * sx] }
  }

  const pickAt = (px, py) => {
    let best = null
    for (const mk of marks) {
      const [sx, sy] = screenOf(mk).at
      const d = Math.hypot(sx - px, sy - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: mk.collection, id: mk.obj.id }
    }
    if (best) return best
    for (const s of switchDots) {
      const [sx, sy] = project(s.ll)
      const d = Math.hypot(sx - px, sy - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: 'switches', id: s.id }
    }
    return best
  }

  /** The place on the nearest track within reach of a point of the screen, or null. */
  const snapAt = (px, py) => {
    const ll = m.unproject([px, py])
    return locate(drawing, drawing.frame.toLocal([ll.lng, ll.lat]), (PICK_PX * 1.6) / scale + 2)
  }
  // Placing: the mouse snaps to a track, and what would be set is drawn there.
  handlers.current.move = (px, py) => {
    if (!m || mode?.kind !== 'place') return
    const hit = px == null ? null : snapAt(px, py)
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
    const hit = pickAt(px, py)
    if (mode?.kind === 'pick-points') {
      if (hit?.collection === 'detectionPoints') onPickPoint?.(hit.id)
      return
    }
    onPick?.(hit ? { collection: hit.collection, id: hit.id } : null)
  }
  useEffect(() => {
    if (m) m.getCanvas().style.cursor = mode ? (mode.kind === 'place' && !(hover && mode.make(hover)) ? 'not-allowed' : 'crosshair') : ''
  }, [m, mode, hover])
  useEffect(() => { if (mode?.kind !== 'place') setHover(null) }, [mode])

  // What placing would set where the mouse is, drawn as the plan draws it.
  const preview = useMemo(() => {
    if (mode?.kind !== 'place' || !hover) return null
    const obj = mode.make(hover)
    if (!obj) return null
    const p = placeOf(drawing, locOf(mode.collection, obj))
    return p ? { collection: mode.collection, obj, p, ll: drawing.frame.toLngLat(p.xy) } : null
  }, [mode, hover, drawing])

  const isSel = (collection, id) => selection?.collection === collection && selection.id === id
  const path = (pts) => pts.map((p, i) => { const [x, y] = project(p); return `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}` }).join('')
  const llOf = (pts) => pts.map(p => drawing.frame.toLngLat(p))
  const showNames = scale > 0.6

  return (
    <div ref={box} className="plan">
      <div className="plan-map"><div ref={mapBox} className="plan-map-inner" /></div>
      {m && (
        <svg className="plan-overlay" role="img" aria-label={t('section_signals')}>
          <g>{lines.map(l => <path key={l.id} d={path(l.ll)} className="plan-track" />)}</g>
          <g>
            {(highlight?.runs ?? []).map((r, i) => <path key={`r${i}`} d={path(lines.find(l => l.id === r)?.ll ?? [])} className="plan-run" />)}
            {(highlight?.stretches ?? []).map((s, i) => (
              <path key={`s${i}`} d={path(llOf(stretchLine(drawing, s.trackId, s.from, s.to)))} className={`plan-stretch${s.tone ? ` plan-stretch-${s.tone}` : ''}`} />
            ))}
          </g>
          <g>
            {switchDots.map(s => {
              const [x, y] = project(s.ll)
              return <circle key={s.id} cx={x} cy={y} r={isSel('switches', s.id) ? 5 : 3} className={`plan-switch${isSel('switches', s.id) ? ' is-selected' : ''}`} />
            })}
          </g>
          <g>
            {marks.filter(mk => !(preview && mk.obj.id === preview.obj.id)).map(mk => {
              const sel = isSel(mk.collection, mk.obj.id) || highlight?.ids?.includes(mk.obj.id)
              return <Placed key={`${mk.collection}:${mk.obj.id}`} mk={mk} screen={screenOf(mk)} sel={sel} showName={showNames} t={t} />
            })}
          </g>
          {preview?.collection === 'speedSections' && (
            <g className="plan-preview">
              {runStretches(net, preview.obj.start, preview.obj.path.trackIds, preview.obj.end).map((st, i) => (
                <path key={i} d={path(llOf(stretchLine(drawing, st.trackId, st.from, st.to)))} className="plan-stretch" />
              ))}
            </g>
          )}
          {preview && (
            <g className="plan-preview">
              <circle cx={screenOf(preview).on[0]} cy={screenOf(preview).on[1]} r="4" className="plan-snap" />
              <Placed mk={preview} screen={screenOf(preview)} sel showName={false} t={t} />
            </g>
          )}
        </svg>
      )}
    </div>
  )
}

/** Where an object of a collection stands: its location, a derailer's its own, a speed section's its start. */
const locOf = (collection, obj) => (collection === 'switchElements' ? obj.derailer?.at : collection === 'speedSections' ? obj.start : obj.at)

/** An object on the plan: a dot on its track, a stroke out to its symbol, the symbol, its name. */
function Placed({ mk, screen, sel, showName, t }) {
  const [x, y] = screen.at
  const [ox, oy] = screen.on
  const apart = Math.hypot(x - ox, y - oy) > 2
  return (
    <g className={`plan-mark${sel ? ' is-selected' : ''}${mk.p.orphan ? ' is-orphan' : ''}`}>
      {apart && <><path d={`M${ox.toFixed(1)},${oy.toFixed(1)}L${x.toFixed(1)},${y.toFixed(1)}`} className="plan-stem" /><circle cx={ox} cy={oy} r="1.6" className="plan-foot" /></>}
      <g transform={`translate(${x.toFixed(1)},${y.toFixed(1)})`}>
        <Mark collection={mk.collection} obj={mk.obj} angle={mk.p.angle ?? 0} />
        {showName && mk.collection === 'signals' && mk.obj.name && <text x="9" y="-7" className="plan-label">{labelOf('signals', mk.obj, t)}</text>}
      </g>
    </g>
  )
}

/** One symbol, pointing the way it acts: `angle` in degrees from north, clockwise. */
function Mark({ collection, obj, angle }) {
  const rot = `rotate(${(angle - 90).toFixed(1)})`
  if (collection === 'detectionPoints') {
    return <g transform={rot}><path d="M0,-6 L0,6" className="plan-dp" /><circle r="2.2" className="plan-dp-dot" /></g>
  }
  if (collection === 'balises') {
    // A PZB magnet a square in the colour of its frequency, an ETCS balise a diamond.
    if (obj.system === 'etcs') return <g transform={rot}><path d="M0,-4 L4,0 L0,4 L-4,0 Z" className="plan-balise plan-balise-etcs" /></g>
    const frequency = obj.system === 'pzb' ? String(obj.settings?.frequency ?? '').replace('+', '-') : 'other'
    return <g transform={rot}><rect x="-3" y="-3" width="6" height="6" className={`plan-balise plan-balise-pzb-${frequency}`} /></g>
  }
  if (collection === 'speedSections') {
    // A speed board at its start, upright, its number in tens as Lf 7 shows it.
    return <g><rect x="-7" y="-6" width="14" height="12" rx="1.5" className="plan-speed" /><text y="3.5" className="plan-speed-text">{obj.speed != null ? Math.round(obj.speed / 10) : '?'}</text></g>
  }
  if (collection === 'switchElements') {
    return <g transform={rot}><path d="M-4,-4 L4,4 M-4,4 L4,-4" className="plan-derailer" /></g>
  }
  const tone = signalTone(obj)
  if (tone === 'virtual') return <g transform={rot}><circle r="4" className="plan-virtual" /></g>
  if (tone === 'board' || tone === 'marker') {
    return <g transform={rot}><rect x="-3.5" y="-3.5" width="7" height="7" fill={TONE_FILL[tone]} className="plan-board" /></g>
  }
  // A signal: a short mast, its head ahead in the way it acts.
  return (
    <g transform={rot}>
      <path d="M-6,0 L2,0" className="plan-mast" />
      <circle cx="6" cy="0" r="4" fill={TONE_FILL[tone]} className="plan-head" />
    </g>
  )
}
