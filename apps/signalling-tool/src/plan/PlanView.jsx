import { useEffect, useMemo, useRef, useState } from 'react'
import { locate, placeOf, stretchLine } from '../model/net.js'
import { signalTone, TONE_FILL } from './symbols.js'
import { labelOf } from '../model/domain.js'

/**
 * The plan: the net in its true position, every track a line, every switch a
 * dot, and on it what the domain holds — signals by their kind on the side
 * they stand, boards, detection points, PZB magnets. What is chosen is drawn
 * in the primary colour, with its run, its section or its overlap. A wheel
 * zooms about the pointer, a drag moves the plan, a click picks what is
 * nearest — or, while placing, the location on the track clicked.
 */

const PICK_PX = 12
const SIDE_PX = 9

const toPath = (pts, view) => pts.map((p, i) => `${i ? 'L' : 'M'}${view.x(p[0]).toFixed(1)},${view.y(p[1]).toFixed(1)}`).join('')

export default function PlanView({ drawing, net, domain, selection, highlight, mode, onPick, onPlace, onPickPoint, fitKey, focus, t }) {
  const box = useRef(null)
  const [size, setSize] = useState({ w: 800, h: 600 })
  const [cam, setCam] = useState(null)   // { cx, cy, scale, fitKey, drawing }: the frame's point at the middle, px per metre
  const drag = useRef(null)

  useEffect(() => {
    const el = box.current
    if (!el) return undefined
    const measure = () => setSize({ w: el.clientWidth || 800, h: el.clientHeight || 600 })
    measure()
    if (typeof ResizeObserver === 'undefined') return undefined
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // The whole net in view: until the plan is moved, and again whenever `fitKey` changes.
  const fitted = useMemo(() => {
    const [x0, y0, x1, y1] = drawing.bounds
    const scale = Math.min(size.w / Math.max(x1 - x0, 20), size.h / Math.max(y1 - y0, 20)) * 0.9
    return { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, scale }
  }, [drawing, size])
  const moved = cam && cam.fitKey === fitKey && cam.drawing === drawing ? cam : null
  // An object chosen in a list is brought into view: the plan centred on it,
  // close enough to tell it from its neighbours — until the plan is moved.
  const focused = useMemo(() => {
    if (!focus || cam?.focusN === focus.n) return null
    const p = placeOf(drawing, focus.at)
    if (!p) return null
    const base = moved ?? fitted
    return { cx: p.xy[0], cy: p.xy[1], scale: Math.max(base.scale, 1.2) }
  }, [focus, cam, drawing, moved, fitted])
  const setView = (next) => setCam({ ...next, fitKey, drawing, focusN: focus?.n })

  const c = focused ?? moved ?? fitted
  const view = useMemo(() => ({
    x: (mx) => size.w / 2 + (mx - c.cx) * c.scale,
    y: (my) => size.h / 2 - (my - c.cy) * c.scale,
    toFrame: (px, py) => [c.cx + (px - size.w / 2) / c.scale, c.cy - (py - size.h / 2) / c.scale],
  }), [c.cx, c.cy, c.scale, size])

  // What is drawn at a point of the domain, and the screen place of each.
  const marks = useMemo(() => {
    const out = []
    const put = (collection, obj, loc, extra = {}) => {
      const p = placeOf(drawing, loc)
      if (p) out.push({ collection, obj, p, ...extra })
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
      if (!a) continue
      const t = drawing.tracks.get(a.trackId)
      if (!t) continue
      const p = placeOf(drawing, { anchor: 'station', trackId: a.trackId, station: a.endpoint === 'BEGIN' ? 0 : t.length })
      if (p) out.push({ id: sw.switchId, name: sw.name, xy: p.xy })
    }
    return out
  }, [net, drawing])

  const screenOf = (m) => {
    const side = m.obj.at?.side ?? m.obj.derailer?.at?.side
    const b = ((m.p.bearing ?? m.p.angle) * Math.PI) / 180
    // Right of the stationing: the bearing turned a quarter clockwise; on screen y runs down.
    const sx = side === 'left' ? -1 : side === 'right' ? 1 : 0
    const dx = Math.cos(b) * SIDE_PX * sx, dy = Math.sin(b) * SIDE_PX * sx
    return [view.x(m.p.xy[0]) + dx, view.y(m.p.xy[1]) + dy]
  }

  const pickAt = (px, py) => {
    let best = null
    for (const m of marks) {
      const [sx, sy] = screenOf(m)
      const d = Math.hypot(sx - px, sy - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: m.collection, id: m.obj.id }
    }
    if (best) return best
    for (const s of switchDots) {
      const d = Math.hypot(view.x(s.xy[0]) - px, view.y(s.xy[1]) - py)
      if (d < PICK_PX && (!best || d < best.d)) best = { d, collection: 'switches', id: s.id }
    }
    return best
  }

  const onWheel = (e) => {
    const rect = box.current.getBoundingClientRect()
    const px = e.clientX - rect.left, py = e.clientY - rect.top
    const before = view.toFrame(px, py)
    const scale = Math.max(0.005, Math.min(80, c.scale * (e.deltaY < 0 ? 1.2 : 1 / 1.2)))
    setView({ scale, cx: before[0] - (px - size.w / 2) / scale, cy: before[1] + (py - size.h / 2) / scale })
  }
  useEffect(() => {
    const el = box.current
    if (!el) return undefined
    const stop = (e) => e.preventDefault()
    el.addEventListener('wheel', stop, { passive: false })
    return () => el.removeEventListener('wheel', stop)
  }, [])

  const onPointerDown = (e) => {
    drag.current = { x: e.clientX, y: e.clientY, cam: c, moved: false }
    e.currentTarget.setPointerCapture?.(e.pointerId)
  }
  const onPointerMove = (e) => {
    const d = drag.current
    if (!d || !d.cam) return
    const dx = e.clientX - d.x, dy = e.clientY - d.y
    if (!d.moved && Math.hypot(dx, dy) < 4) return
    d.moved = true
    setView({ cx: d.cam.cx - dx / d.cam.scale, cy: d.cam.cy + dy / d.cam.scale, scale: d.cam.scale })
  }
  const onPointerUp = (e) => {
    const d = drag.current
    drag.current = null
    if (!d || d.moved) return
    const rect = box.current.getBoundingClientRect()
    const px = e.clientX - rect.left, py = e.clientY - rect.top
    if (mode?.kind === 'place') {
      const hit = locate(drawing, view.toFrame(px, py), PICK_PX / c.scale + 2)
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

  const isSel = (collection, id) => selection?.collection === collection && selection.id === id
  const showNames = c.scale > 0.6

  return (
    <div ref={box} className={`plan${mode ? ' plan-placing' : ''}`} onWheel={onWheel}
      onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp}>
      <svg width={size.w} height={size.h} role="img" aria-label={t('section_signals')}>
        <g className="plan-tracks">
          {[...drawing.tracks.values()].map(tr => (
            <path key={tr.id} d={toPath(tr.line, view)} className="plan-track" />
          ))}
        </g>
        <g className="plan-highlight">
          {(highlight?.runs ?? []).map((r, i) => (
            <path key={`r${i}`} d={toPath(drawing.tracks.get(r)?.line ?? [], view)} className="plan-run" />
          ))}
          {(highlight?.stretches ?? []).map((s, i) => (
            <path key={`s${i}`} d={toPath(stretchLine(drawing, s.trackId, s.from, s.to), view)} className={`plan-stretch${s.tone ? ` plan-stretch-${s.tone}` : ''}`} />
          ))}
        </g>
        <g className="plan-switches">
          {switchDots.map(s => (
            <circle key={s.id} cx={view.x(s.xy[0])} cy={view.y(s.xy[1])} r={isSel('switches', s.id) ? 5 : 3}
              className={`plan-switch${isSel('switches', s.id) ? ' is-selected' : ''}`} />
          ))}
        </g>
        <g className="plan-marks">
          {marks.map(m => {
            const [sx, sy] = screenOf(m)
            const sel = isSel(m.collection, m.obj.id) || highlight?.ids?.includes(m.obj.id)
            const angle = m.p.angle ?? 0
            return (
              <g key={`${m.collection}:${m.obj.id}`} transform={`translate(${sx.toFixed(1)},${sy.toFixed(1)})`}
                className={`plan-mark${sel ? ' is-selected' : ''}${m.p.orphan ? ' is-orphan' : ''}`}>
                <Mark collection={m.collection} obj={m.obj} angle={angle} />
                {showNames && m.collection === 'signals' && m.obj.name && (
                  <text x="9" y="-7" className="plan-label">{labelOf('signals', m.obj, t)}</text>
                )}
              </g>
            )
          })}
        </g>
      </svg>
      <div className="plan-scale" aria-hidden="true">{scaleText(c.scale)}</div>
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

/** A scale bar's words: the round length that is about 100 px long. */
function scaleText(scale) {
  const m = 100 / scale
  const steps = [1, 2, 5]
  let unit = 10 ** Math.floor(Math.log10(m))
  let best = unit
  for (const s of steps) if (s * unit <= m) best = s * unit
  return best >= 1000 ? `${best / 1000} km` : `${best} m`
}
