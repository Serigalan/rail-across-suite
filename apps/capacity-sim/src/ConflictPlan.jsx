import { useEffect, useMemo, useRef, useState } from 'react'
import { schematicLayout, schematicPlacer, schematicStrip } from '@rail-across/core/schematic'
import { areaColor, BINDING_COLOR } from './plan.js'

/**
 * The schematic plan of a stretch (decision 0014): the variant's net
 * as the core's schematic strip — its lines lanes along the longest, every
 * switch a diagonal —, on it the conflict areas coloured by how many trains
 * hold them and the binding elements of the critical chain, each with its
 * number. It shows the stretch's areas first; the wheel zooms along it, a
 * drag moves it.
 */

/** The drawing's width, in its units; it is shown at the width of its box. */
const W = 1000
/** Spacing of two lanes, and the margin above and below [units]. */
const GAP = 34
const PAD = 40
/** How much of the areas' width is added on either side, fitting them. */
const MARGIN = 0.15

export default function ConflictPlan({ record, stations, marks, t }) {
  const layout = useMemo(() => (record?.tracks?.length
    ? schematicLayout({ tracks: record.tracks, switches: record.switches ?? [], platforms: record.platforms ?? [] })
    : null), [record])
  // The lanes in use, closed up about y = 0.
  const { laneY, span } = useMemo(() => {
    if (!layout) return { laneY: () => 0, span: 0 }
    const used = new Set()
    for (const st of layout.strands) if (st.lane != null && !st.connector) for (const sc of st.sections) if (sc.lane != null) used.add(sc.lane)
    for (const n of layout.nodes) if (n.lane != null) used.add(n.lane)
    const lanes = [...used].sort((a, b) => a - b)
    return {
      laneY: (lane) => {
        let rank = lanes.indexOf(lane)
        if (rank < 0) rank = lanes.filter(l => l < lane).length - 0.5
        return -(rank - (lanes.length - 1) / 2) * GAP
      },
      span: Math.max(lanes.length - 1, 0) * GAP,
    }
  }, [layout])
  // What the areas reach along the strip, fitted with a margin; the whole strip where there are none.
  const fit = useMemo(() => {
    if (!layout) return [0, 1]
    const xs = [...marks.areas, ...marks.binding].flatMap(m => m.stretches)
      .flatMap(s => [layout.stationX(s.trackId, s.lo), layout.stationX(s.trackId, s.hi)]).filter(Number.isFinite)
    if (!xs.length) return layout.extent
    const [a, b] = [Math.min(...xs), Math.max(...xs)]
    const m = Math.max((b - a) * MARGIN, 100)
    return [a - m, b + m]
  }, [layout, marks])
  const fitKey = fit.join('|')
  const [view, setView] = useState({ key: null, range: fit })
  const range = view.key === fitKey ? view.range : fit
  const strip = useMemo(() => (layout
    ? schematicStrip(layout, {
      laneY, dxOf: (dy) => Math.abs(dy) * (fit[1] - fit[0]) / W, leg: 8, gap: GAP,
      show: { trackNames: false }, sizes: { platformNear: 0.25, platformFar: 0.6 },
    })
    : null), [layout, laneY, fit])
  const place = useMemo(() => (strip ? schematicPlacer(layout, strip) : null), [layout, strip])
  const drag = useRef(null)
  const svg = useRef(null)
  const shown = useRef(range)
  useEffect(() => { shown.current = range })
  // The wheel zooms about the pointer, not the page: a listener that may keep the page from scrolling.
  useEffect(() => {
    const el = svg.current
    if (!el) return undefined
    const zoom = (e) => {
      e.preventDefault()
      const r = shown.current
      const box = el.getBoundingClientRect()
      const at = r[0] + ((e.clientX - box.left) / box.width) * (r[1] - r[0])
      const k = e.deltaY > 0 ? 1.25 : 0.8
      setView({ key: fitKey, range: [at - (at - r[0]) * k, at + (r[1] - at) * k] })
    }
    el.addEventListener('wheel', zoom, { passive: false })
    return () => el.removeEventListener('wheel', zoom)
  }, [fitKey, place])
  if (!layout || !place) return <p className="ui-muted">{t('plan_none')}</p>

  const H = span + 2 * PAD + 20
  const X = (x) => ((x - range[0]) / (range[1] - range[0])) * W
  const Y = (y) => y + PAD + 20 + span / 2
  const pts = (list) => list.map(([x, y]) => `${X(x).toFixed(1)},${Y(y).toFixed(1)}`).join(' ')
  const lineOf = (s) => place.stretch(s.trackId, s.lo, s.hi)
  const down = (e) => { drag.current = { x: e.clientX, range, width: e.currentTarget.getBoundingClientRect().width } }
  const move = (e) => {
    const d = drag.current
    if (!d) return
    const dx = ((e.clientX - d.x) / d.width) * (d.range[1] - d.range[0])
    setView({ key: fitKey, range: [d.range[0] - dx, d.range[1] - dx] })
  }
  // The places' names above the lanes, where the strip has a track of theirs.
  const names = (stations ?? []).filter(s => s.kind !== 'end').map(s => {
    const tr = s.tracks[0] ?? (s.platforms[0] && { id: s.platforms[0].trackId, at: s.platforms[0].at })
    const p = tr && place.at(tr.id, tr.at ?? (tr.length ?? 0) / 2)
    return p ? { name: s.name, x: X(p.x) } : null
  }).filter(n => n && n.x > -50 && n.x < W + 50)

  return (
    <div className="cap-plan">
      <svg ref={svg} viewBox={`0 0 ${W} ${H}`} onPointerDown={down} onPointerMove={move} onPointerUp={() => { drag.current = null }}
        onPointerLeave={() => { drag.current = null }} role="img" aria-label={t('plan')}>
        <rect width={W} height={H} fill="transparent" />
        {strip.items.map((it, i) => {
          if (it.kind === 'line') return <polyline key={i} points={pts(it.pts)} fill="none" stroke="#9aa0ad" strokeWidth={1.5} />
          if (it.kind === 'fill') return <polygon key={i} points={pts(it.pts)} fill="#9aa0ad" />
          if (it.kind === 'platform') return <rect key={i} x={Math.min(X(it.x0), X(it.x1))} y={Y(it.y0)} width={Math.abs(X(it.x1) - X(it.x0))} height={Math.max(Y(it.y1) - Y(it.y0), 1)} fill="#dfe3ea" />
          return null
        })}
        {marks.areas.flatMap(a => a.stretches.map((s, j) => (
          <polyline key={`${a.key}-${j}`} points={pts(lineOf(s))} fill="none" stroke={areaColor(a.trains)} strokeWidth={7} strokeOpacity={0.75} strokeLinecap="round">
            <title>{t('area_title', { n: a.trains })}</title>
          </polyline>
        )))}
        {marks.binding.flatMap(b => b.stretches.map((s, j) => (
          <polyline key={`b-${b.after}-${j}`} points={pts(lineOf(s))} fill="none" stroke={BINDING_COLOR} strokeWidth={3.5} strokeLinecap="round" />
        )))}
        {[...new Map(marks.binding.map(b => [b.n, b])).values()].map(b => {
          const s = b.stretches[0]
          const p = s && place.at(s.trackId, (s.lo + s.hi) / 2)
          if (!p) return null
          return (
            <g key={`n-${b.n}`} transform={`translate(${X(p.x)},${Y(p.y) - 14})`}>
              <circle r={8} fill={BINDING_COLOR} />
              <text textAnchor="middle" dy={4} fill="#fff" fontSize={10} fontWeight={600}>{b.n}</text>
            </g>
          )
        })}
        {names.map((n, i) => <text key={i} x={n.x} y={14} textAnchor="middle" className="cap-plan-name">{n.name}</text>)}
      </svg>
      <div className="cap-plan-legend">
        {[2, 3, 4].map(n => <span key={n}><i style={{ background: areaColor(n) }} />{t(n >= 4 ? 'legend_area_more' : 'legend_area_n', { n })}</span>)}
        <span><i style={{ background: BINDING_COLOR }} />{t('legend_binding')}</span>
        <span className="ui-muted">{t('plan_hint')}</span>
      </div>
    </div>
  )
}
