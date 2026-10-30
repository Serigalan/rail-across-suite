import { useEffect, useMemo, useRef, useState } from 'react'
import { findBasemap, isElevationBasemap, updateElevationRange } from '@rail-across/basemaps'
import { locate, placeOf, stretchLine } from '../model/net.js'
import { signalTone, TONE_FILL } from './symbols.js'
import { labelOf } from '../model/domain.js'

/**
 * The plan: the net in its true position on the basemap chosen in the layers
 * panel (the suite's basemaps, as the Layout-Tool has them), every track a
 * line, every switch a dot, and on it what the domain holds — signals by
 * their kind on the side they stand, boards, detection points, PZB magnets.
 * What is chosen is drawn in the primary colour, with its run, its section or
 * its overlap. The map moves and zooms (MapLibre, north up); the plan is
 * drawn over it in screen pixels, so a symbol stays readable at every zoom. A
 * click picks what is nearest — or, while placing, the location on the track
 * clicked.
 */

const PICK_PX = 12
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
    for (const z of domain.collections.pzbElements) put('pzbElements', z, z.at)
    for (const e of domain.collections.switchElements) if (e.derailer) put('switchElements', e, e.derailer.at)
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

  const screenOf = (mk) => {
    const [x, y] = project(mk.ll)
    const side = mk.obj.at?.side ?? mk.obj.derailer?.at?.side
    const b = ((mk.p.bearing ?? mk.p.angle) * Math.PI) / 180
    // Right of the stationing: the bearing turned a quarter clockwise; on screen y runs down.
    const sx = side === 'left' ? -1 : side === 'right' ? 1 : 0
    return [x + Math.cos(b) * SIDE_PX * sx, y + Math.sin(b) * SIDE_PX * sx]
  }

  const pickAt = (px, py) => {
    let best = null
    for (const mk of marks) {
      const [sx, sy] = screenOf(mk)
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

  // What a click does, as it is now (the map's listener is made once).
  handlers.current.click = (px, py) => {
    if (!m) return
    if (mode?.kind === 'place') {
      const ll = m.unproject([px, py])
      const hit = locate(drawing, drawing.frame.toLocal([ll.lng, ll.lat]), PICK_PX / scale + 2)
      if (hit) onPlace?.(hit)
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
    if (m) m.getCanvas().style.cursor = mode ? 'crosshair' : ''
  }, [m, mode])

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
            {marks.map(mk => {
              const [sx, sy] = screenOf(mk)
              const sel = isSel(mk.collection, mk.obj.id) || highlight?.ids?.includes(mk.obj.id)
              return (
                <g key={`${mk.collection}:${mk.obj.id}`} transform={`translate(${sx.toFixed(1)},${sy.toFixed(1)})`}
                  className={`plan-mark${sel ? ' is-selected' : ''}${mk.p.orphan ? ' is-orphan' : ''}`}>
                  <Mark collection={mk.collection} obj={mk.obj} angle={mk.p.angle ?? 0} />
                  {showNames && mk.collection === 'signals' && mk.obj.name && <text x="9" y="-7" className="plan-label">{labelOf('signals', mk.obj, t)}</text>}
                </g>
              )
            })}
          </g>
        </svg>
      )}
    </div>
  )
}

/** One symbol, pointing the way it acts: `angle` in degrees from north, clockwise. */
function Mark({ collection, obj, angle }) {
  const rot = `rotate(${(angle - 90).toFixed(1)})`
  if (collection === 'detectionPoints') {
    return <g transform={rot}><path d="M0,-6 L0,6" className="plan-dp" /><circle r="2.2" className="plan-dp-dot" /></g>
  }
  if (collection === 'pzbElements') {
    return <g transform={rot}><rect x="-3" y="-3" width="6" height="6" className={`plan-pzb plan-pzb-${obj.frequency?.replace('+', '-')}`} /></g>
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
