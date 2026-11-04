import { useEffect, useMemo, useRef, useState } from 'react'
import { schematicLayout, schematicPlacer, schematicStrip } from '@rail-across/core/schematic'
import { STATUS_COLOR } from '@rail-across/core/track'
import { labelOf } from '../model/domain.js'
import { BaliseSymbol, DerailerSymbol, DetectionSymbol, SANS, SignalSymbol } from './planzeichen.jsx'
import { signalSymbolOf, symbolLength } from './symbols.js'

/**
 * The schematic view: the net as the strip of the core's schematic layout —
 * its lines lanes along the longest of them, every switch a 45° connection,
 * the length along the lanes to scale — and on it the signalling in the
 * symbols of a signalling plan (planzeichen.jsx), the stopping places bands
 * under their tracks. It shows, picks and highlights as the site plan does; setting anew is the site plan's.
 *
 * It is drawn once to one scale both ways — a metre along the lanes one unit
 * (METRE), two lanes GAP apart, every symbol and text its size — and shown
 * zoomed as a whole: zoomed out, all of it is smaller together, its 45°
 * connections and its proportions as they are. Only a track keeps a hairline
 * at the least, so the net stays in sight however far out. Wheel zooms, a
 * drag moves.
 */

/** A metre along the lanes, in the drawing's units — a pixel each at a zoom of 1. */
const METRE = 1
/** Spacing of two lanes [units]. */
const GAP = 52
/** From a track to the axis of a symbol beside it [units]. */
const SIDE = 12
/** How far from a symbol a click still picks it [px on the screen]. */
const PICK_PX = 14
const LEG = 6
/** How far from the reference a track may lie and still be drawn [m]. */
const CORRIDOR = 1000
const MIN_ZOOM = 0.002
const MAX_ZOOM = 20
/** The zoom an object chosen in a list is shown at, the least: its symbols at their size. */
const CLOSE_ZOOM = 1
const zoomed = (z) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z))

/** Which way, on the screen, a side of a track lies: 1 below, -1 above, 0 on it. */
const sideY = (side, dir) => (side === 'left' ? -dir : side === 'right' ? dir : 0)

export default function SchematicView({ net, platforms, domain, selection, highlight, pending, onPick, fitKey, focus, t }) {
  const box = useRef(null)
  const [size, setSize] = useState(null)   // the box's, once measured
  const [view, setView] = useState(null)   // { z: the zoom [px/unit], tx, ty: where the drawing's origin is [px] }
  const drag = useRef(null)

  useEffect(() => {
    const el = box.current
    if (!el) return undefined
    const measure = () => setSize({ w: el.clientWidth || 800, h: el.clientHeight || 600 })
    measure()
    const watch = new ResizeObserver(measure)
    watch.observe(el)
    return () => watch.disconnect()
  }, [])

  const layout = useMemo(
    () => schematicLayout({ tracks: net.tracks, switches: net.switches, platforms: platforms ?? [], corridor: CORRIDOR }),
    [net, platforms],
  )
  // The lanes in use, closed up: neighbouring tracks are one gap apart, the lanes about y = 0.
  const { laneY, laneSpan } = useMemo(() => {
    if (!layout) return { laneY: () => 0, laneSpan: 0 }
    const used = new Set()
    for (const st of layout.strands) {
      if (st.lane != null && !st.connector) for (const sc of st.sections) if (sc.lane != null) used.add(sc.lane)
    }
    for (const n of layout.nodes) if (n.lane != null) used.add(n.lane)
    const lanes = [...used].sort((a, b) => a - b)
    return {
      laneY: (lane) => {
        let rank = lanes.indexOf(lane)
        if (rank < 0) rank = lanes.filter(l => l < lane).length - 0.5
        return -(rank - (lanes.length - 1) / 2) * GAP
      },
      laneSpan: Math.max(lanes.length - 1, 0) * GAP,
    }
  }, [layout])

  // The whole strip in view, both ways: when it is first there, and again on asking.
  const measured = size != null
  useEffect(() => {
    if (!layout || !measured) return
    const [xa, xb] = layout.extent
    const z = zoomed(Math.min((size.w - 120) / Math.max((xb - xa) * METRE, 1), (size.h - 80) / (laneSpan + 2 * GAP)))
    setView({ z, tx: size.w / 2 - ((xa + xb) / 2) * METRE * z, ty: size.h / 2 })
    // The size is read when fitting, not followed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layout, fitKey, measured])

  // Drawn once to its scale: zooming shows it larger or smaller, it does not lay it out anew.
  const strip = useMemo(() => (layout
    ? schematicStrip(layout, {
      laneY, dxOf: (dy) => Math.abs(dy) / METRE, leg: LEG, gap: GAP,
      sizes: { trackLabel: 5, switchAbove: 8, switchBelow: 16, platformNear: 0.3, platformFar: 0.55 },
    })
    : null), [layout, laneY])
  const place = useMemo(() => (strip ? schematicPlacer(layout, strip) : null), [layout, strip])

  // An object chosen in a list is brought into view, close enough to tell it
  // from its neighbours; what reaches further — a way — as a whole.
  useEffect(() => {
    if (!layout || !measured || !focus) return
    if (focus.fit) {
      const xs = focus.fit.map(loc => layout.stationX(loc.trackId, loc.station)).filter(x => x != null)
      if (!xs.length) return
      const [xa, xb] = [Math.min(...xs), Math.max(...xs)]
      const z = zoomed(Math.min((size.w - 160) / Math.max((xb - xa) * METRE, 1), (size.h - 80) / (laneSpan + 2 * GAP)))
      setView({ z, tx: size.w / 2 - ((xa + xb) / 2) * METRE * z, ty: size.h / 2 })
      return
    }
    const at = focus.at
    if (at?.anchor !== 'station') return
    const x = layout.stationX(at.trackId, at.station)
    if (x == null) return
    setView(v => {
      const z = Math.max(v?.z ?? CLOSE_ZOOM, CLOSE_ZOOM)
      const lane = layout.sectionOfTrack.get(at.trackId)?.lane
      return { z, tx: size.w / 2 - x * METRE * z, ty: size.h / 2 - (lane != null ? laneY(lane) : 0) * z }
    })
    // Only a new focus moves the view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focus])

  // Every object of the domain where the strip has its track: its point, its side, which way it acts.
  const marks = useMemo(() => {
    if (!place) return []
    const out = []
    const put = (collection, obj, loc, extra = {}) => {
      if (loc?.anchor !== 'station') return
      const p = place.at(loc.trackId, loc.station)
      if (!p) return
      const sy = sideY(loc.side, p.dir)
      // Along x the way it acts: with the stationing (up) or against it.
      const ax = (loc.direction === 'down' ? -1 : 1) * p.dir
      out.push({ collection, obj, x: p.x, y: p.y, sy, ax, ...extra })
    }
    for (const s of domain.collections.signals) put('signals', s, s.at, { symbol: signalSymbolOf(s) })
    for (const d of domain.collections.detectionPoints) put('detectionPoints', d, d.at)
    for (const b of domain.collections.balises) put('balises', b, b.at)
    for (const e of domain.collections.switchElements) if (e.derailer) put('switchElements', e, e.derailer.at)
    return out
  }, [domain, place])

  if (!view || !layout || !strip) return <div ref={box} className="schema" />
  const { z } = view
  // In the drawing's units; the group they are drawn in zooms them.
  const X = (x) => x * METRE
  const Y = (y) => y
  const pts = (list) => list.map(([x, y], i) => `${i ? 'L' : 'M'}${X(x).toFixed(1)},${Y(y).toFixed(1)}`).join('')
  const isSel = (collection, id) => (selection?.collection === collection && selection.id === id) || !!highlight?.ids?.includes(id)

  /** Where a mark's symbol is drawn on the screen: on its track, or beside it on its side. */
  const symbolAt = (mk) => {
    if (mk.collection === 'signals') return [X(mk.x), Y(mk.y) + (mk.sy || 1) * SIDE]
    return [X(mk.x), Y(mk.y)]
  }
  const switchesAt = strip.items.filter(i => (i.kind === 'switch' || i.kind === 'dot' || (i.kind === 'label' && i.switch)) && i.node)
  /** What a click picks: in the drawing's units, within PICK_PX on the screen. */
  const pickAt = (qx, qy) => {
    const px = (qx - view.tx) / z, py = (qy - view.ty) / z
    const reach = PICK_PX / z
    let best = null
    for (const mk of marks) {
      const [sx, sy] = symbolAt(mk)
      // A signal is picked along its whole symbol, not only at its foot.
      const len = mk.symbol ? symbolLength(mk.symbol) : 0
      const cx = sx + (mk.ax * len) / 2
      const d = Math.max(0, Math.abs(px - cx) - len / 2) + Math.abs(py - sy)
      if (d < reach && (!best || d < best.d)) best = { d, collection: mk.collection, id: mk.obj.id }
    }
    if (best) return best
    for (const it of switchesAt) {
      const d = Math.hypot(px - X(it.x), py - Y(laneY(it.node.lane)))
      if (d < reach && (!best || d < best.d)) best = { d, collection: 'switches', id: it.node.sw.switchId }
    }
    if (best) return best
    // A stopping place along its band.
    for (const h of domain.collections.stoppingPlaces) {
      const line = place.stretch(h.at.trackId, h.at.startStation, h.at.endStation).map(([x, y]) => [X(x), Y(y)])
      for (let i = 1; i < line.length; i++) {
        const d = toSegment(px, py, line[i - 1], line[i])
        if (d < (PICK_PX / 2 + 2) / z && (!best || d < best.d)) best = { d, collection: 'stoppingPlaces', id: h.id }
      }
    }
    return best
  }

  const onPointerDown = (e) => {
    drag.current = { x: e.clientX, y: e.clientY, tx: view.tx, ty: view.ty, moved: false }
    e.currentTarget.setPointerCapture?.(e.pointerId)
  }
  const onPointerMove = (e) => {
    const d = drag.current
    if (!d) return
    const dx = e.clientX - d.x, dy = e.clientY - d.y
    if (!d.moved && Math.hypot(dx, dy) < 3) return
    d.moved = true
    setView(v => ({ ...v, tx: d.tx + dx, ty: d.ty + dy }))
  }
  const onPointerUp = (e) => {
    const d = drag.current
    drag.current = null
    if (!d || d.moved) return
    const r = box.current.getBoundingClientRect()
    const hit = pickAt(e.clientX - r.left, e.clientY - r.top)
    onPick?.(hit ? { collection: hit.collection, id: hit.id } : null)
  }
  const onWheel = (e) => {
    const r = box.current.getBoundingClientRect()
    const px = e.clientX - r.left, py = e.clientY - r.top
    const k = Math.exp(-e.deltaY * 0.0015)
    // About the pointer, both ways: what is under it stays there.
    setView(v => {
      const next = zoomed(v.z * k)
      return { z: next, tx: px - ((px - v.tx) / v.z) * next, ty: py - ((py - v.ty) / v.z) * next }
    })
  }

  return (
    <div ref={box} className="schema" onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp}
      onWheel={onWheel}>
      <svg className="schema-svg" role="img" aria-label={t('view_schema')} style={{ '--schema-zoom': z }}>
        <g transform={`translate(${view.tx.toFixed(1)} ${view.ty.toFixed(1)}) scale(${z})`}>
          <g className="schema-places">
            {domain.collections.stoppingPlaces.map(h => {
              const line = place.stretch(h.at.trackId, h.at.startStation, h.at.endStation)
              return line.length > 1
                ? <path key={h.id} d={pts(line)} className={`schema-place${isSel('stoppingPlaces', h.id) ? ' is-selected' : ''}`} />
                : null
            })}
          </g>
          <Highlight highlight={highlight} strip={strip} place={place} pts={pts} />
          <g className="schema-net">
            {strip.items.map((it, i) => {
              if (it.kind === 'line') return <path key={i} d={pts(it.pts)} className="schema-track" style={statusStyle(it.status)} />
              if (it.kind === 'fill') return <path key={i} d={`${pts(it.pts)}Z`} className="schema-wedge" style={statusFill(it.status)} />
              if (it.kind === 'platform') {
                return <rect key={i} x={X(it.x0)} y={Y(it.y0)} width={Math.max(0, X(it.x1) - X(it.x0))} height={it.y1 - it.y0} className="schema-platform" />
              }
              return null
            })}
            {switchesAt.map((it, i) => {
              const sel = isSel('switches', it.node.sw.switchId)
              const x = X(it.x), y = Y(laneY(it.node.lane))
              if (it.kind === 'dot') return <circle key={`d${i}`} cx={x} cy={y} r={sel ? 4.5 : 2.5} className={`schema-switch${sel ? ' is-selected' : ''}`} />
              if (it.kind === 'switch') {
                // Beyond its triangle, in the angle on the side the branch leaves to.
                return (
                  <text key={`s${i}`} x={x + it.sx * (LEG + 4)} y={it.sy > 0 ? y + 11 : y - 4} textAnchor={it.sx > 0 ? 'start' : 'end'}
                    className={`schema-switch-label${sel ? ' is-selected' : ''}`}>{it.t}</text>
                )
              }
              return <text key={`l${i}`} x={x} y={Y(it.y)} textAnchor="middle" className={`schema-switch-label${sel ? ' is-selected' : ''}`}>{it.t}</text>
            })}
            {strip.items.filter(it => it.kind === 'label' && it.track && it.room * METRE > it.t.length * 6.5 + 10).map((it, i) => (
              <text key={`t${i}`} x={X(it.x)} y={Y(it.y)} textAnchor="middle" className="schema-track-label">{it.t}</text>
            ))}
          </g>
          <g className="schema-marks">
            {marks.map(mk => (
              <Mark key={`${mk.collection}:${mk.obj.id}`} mk={mk} at={symbolAt(mk)} sel={isSel(mk.collection, mk.obj.id)} drafted={!!pending?.has(mk.obj.id)} t={t} />
            ))}
          </g>
        </g>
      </svg>
    </div>
  )
}

/** Distance of a point from a segment [px]. */
function toSegment(px, py, [ax, ay], [bx, by]) {
  const dx = bx - ax, dy = by - ay
  const len2 = dx * dx + dy * dy
  const k = len2 ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len2)) : 0
  return Math.hypot(px - (ax + k * dx), py - (ay + k * dy))
}

const statusStyle = (status) => (status && status !== 'existing' ? { stroke: STATUS_COLOR[status] } : undefined)
const statusFill = (status) => (status && status !== 'existing' ? { fill: STATUS_COLOR[status] } : undefined)

/** What is chosen, under the net: a route's or overlap's tracks, a section's stretches. */
function Highlight({ highlight, strip, place, pts }) {
  if (!highlight) return null
  const runs = new Set(highlight.runs ?? [])
  return (
    <g className="schema-highlight">
      {strip.items.filter(it => it.kind === 'line' && it.trackIds.some(id => runs.has(id))).map((it, i) => (
        <path key={`r${i}`} d={pts(it.pts)} className="schema-run" />
      ))}
      {(highlight.ways ?? []).flatMap((w, i) => w.pieces.map((p, j) => {
        const line = place.stretch(p.trackId, p.from, p.to)
        return line.length > 1 ? <path key={`w${i}-${j}`} d={pts(line)} className={`schema-way${w.strong ? ' is-strong' : ''}`} /> : null
      }))}
      {(highlight.stretches ?? []).map((s, i) => {
        const line = place.stretch(s.trackId, s.from, s.to)
        return line.length > 1 ? <path key={`s${i}`} d={pts(line)} className={`schema-stretch${s.tone ? ` schema-stretch-${s.tone}` : ''}`} /> : null
      })}
    </g>
  )
}

/** One object in its symbol, in the primary colour when chosen or drafted (a preview); a signal's name beyond its screen. */
function Mark({ mk, at: [x, y], sel, drafted, t }) {
  const fg = sel || drafted ? 'var(--color-primary)' : 'var(--plan-mark)'
  const bg = 'var(--ui-surface)'
  const cls = `schema-mark${sel || drafted ? ' is-selected' : ''}${drafted ? ' is-pending' : ''}`
  if (mk.collection === 'signals') {
    const name = mk.obj.name ? labelOf('signals', mk.obj, t) : null
    const len = symbolLength(mk.symbol)
    return (
      <g className={cls}>
        <g transform={`translate(${x.toFixed(1)},${y.toFixed(1)}) scale(${mk.ax},1)`}>
          <SignalSymbol symbol={mk.symbol} ax={mk.ax} fg={fg} bg={bg} />
        </g>
        {name && (
          // Beyond the screen, turned the way the signal acts, as the symbols' texts are.
          <text transform={`translate(${(x + mk.ax * (len + 7)).toFixed(1)},${y.toFixed(1)}) rotate(${-90 * mk.ax})`} textAnchor="middle"
            dominantBaseline="central" className="schema-name" style={{ fontFamily: SANS }}>{name}</text>
        )}
      </g>
    )
  }
  const sy = mk.sy || 1
  return (
    <g className={cls} transform={`translate(${x.toFixed(1)},${y.toFixed(1)})`}>
      {mk.collection === 'detectionPoints' && <DetectionSymbol kind={mk.obj.kind} fg={fg} bg={bg} />}
      {mk.collection === 'balises' && <BaliseSymbol balise={mk.obj} sy={sy} fg={fg} bg={bg} />}
      {mk.collection === 'switchElements' && <DerailerSymbol sy={sy} fg={fg} />}
    </g>
  )
}
