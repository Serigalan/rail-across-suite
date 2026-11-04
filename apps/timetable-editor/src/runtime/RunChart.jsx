import { useEffect, useRef, useState } from 'react'
import { Tabs } from '@rail-across/ui'
import { formatTime } from '@rail-across/running-times'

/**
 * The way along the path, on one axis of distance: above, the run — the
 * speed over the distance (the speed allowed, MRSP, the fastest run and the
 * run with its margin) or the time over the distance —, once it is
 * simulated; under it the height of the tracks along the same stretch, from
 * their height points, open where a track has none. The stops marked, the
 * moment played, and where the mouse is its speed and height.
 */

const PAD = { left: 52, right: 16, top: 14, bottom: 30 }
const RUN_HEIGHT = 200
const PROFILE_HEIGHT = 130

/** Round steps for an axis from `min` to `max`, about `n` of them. */
function ticks(min, max, n) {
  if (!(max > min)) return [min]
  const raw = (max - min) / n
  const mag = 10 ** Math.floor(Math.log10(raw))
  const step = [1, 2, 2.5, 5, 10].map(f => f * mag).find(s => s >= raw)
  const out = []
  for (let v = Math.ceil(min / step) * step; v <= max + 1e-9; v += step) out.push(Math.round(v * 1e6) / 1e6)
  return out
}

function useWidth(ref) {
  const [width, setWidth] = useState(600)
  useEffect(() => {
    const el = ref.current
    if (!el || typeof ResizeObserver === 'undefined') return undefined
    const ro = new ResizeObserver(([entry]) => setWidth(Math.max(240, entry.contentRect.width)))
    ro.observe(el)
    return () => ro.disconnect()
  }, [ref])
  return width
}

/** A value along the path, straight between the points of `xs` and `ys`; null outside them or in a gap. */
function along(xs, ys, s) {
  if (!xs.length || s < xs[0] || s > xs[xs.length - 1]) return null
  let i = 1
  while (i < xs.length - 1 && xs[i] < s) i++
  const a = ys[i - 1], b = ys[i]
  if (a == null || b == null) return null
  const span = xs[i] - xs[i - 1]
  return span > 0 ? a + (b - a) * (s - xs[i - 1]) / span : b
}

/** The pieces of a profile where it has heights: [[{ position, height }]]. */
function pieces(profile) {
  const out = []
  let piece = []
  for (const p of profile) {
    if (p.height == null) { if (piece.length > 1) out.push(piece); piece = [] } else piece.push(p)
  }
  if (piece.length > 1) out.push(piece)
  return out
}

export default function RunChart({ run, profile, length, offsets, labels, at, t, language }) {
  const [view, setView] = useState('speed')
  const [hover, setHover] = useState(null)   // the distance the mouse is at [m]
  const box = useRef(null)
  const width = useWidth(box)
  const w = width - PAD.left - PAD.right
  const x = (s) => PAD.left + (s / length) * w
  const km = length >= 2000
  const xTicks = ticks(0, km ? length / 1000 : length, Math.max(2, Math.floor(w / 90))).map(v => (km ? v * 1000 : v))
  const distance = (s) => (km ? `${(s / 1000).toLocaleString(language, { minimumFractionDigits: 3, maximumFractionDigits: 3 })} km` : `${Math.round(s)} m`)

  const result = run?.result ?? null
  const final = result?.final ?? null
  const speedAt = (s) => (final ? along(final.positions, final.speeds, s) : null)
  const heightAt = (s) => along(profile.map(p => p.position), profile.map(p => p.height), s)
  const onMove = (e) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const s = ((e.clientX - rect.left - PAD.left) / w) * length
    setHover(s >= 0 && s <= length ? s : null)
  }

  const readout = hover == null ? null : [
    distance(hover),
    speedAt(hover) != null && `${Math.round(speedAt(hover) * 3.6)} km/h`,
    heightAt(hover) != null && `${heightAt(hover).toLocaleString(language, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} m`,
  ].filter(Boolean).join(' · ')

  /** What every band has: the grid across, the stops, the moment played and where the mouse is. */
  const across = (h) => <>
    {xTicks.map(v => <line key={`x${v}`} x1={x(v)} x2={x(v)} y1={PAD.top} y2={PAD.top + h} className="sim-grid" />)}
    {offsets.map((s, i) => <line key={`p${i}`} x1={x(s)} x2={x(s)} y1={PAD.top} y2={PAD.top + h} className="sim-stop"><title>{labels[i]}</title></line>)}
    {hover != null && <line x1={x(hover)} x2={x(hover)} y1={PAD.top} y2={PAD.top + h} className="sim-hover" />}
  </>
  const xAxis = (height, h) => <>
    {xTicks.map(v => <text key={`t${v}`} x={x(v)} y={PAD.top + h + 16} textAnchor="middle" className="sim-axis">{km ? (v / 1000).toLocaleString(language) : Math.round(v)}</text>)}
    <text x={PAD.left + w} y={height - 2} textAnchor="end" className="sim-axis">{km ? 'km' : 'm'}</text>
  </>

  return (
    <div className="sim-chart">
      <div className="sim-chart-head">
        {result
          ? <Tabs value={view} onChange={setView} options={[{ value: 'speed', label: t('chart_speed') }, { value: 'time', label: t('chart_time') }]} label={t('chart')} />
          : <span className="sim-chart-title">{t('chart_profile')}</span>}
        {result && <RunLegend view={view} t={t} />}
        {readout && <span className="sim-readout">{readout}</span>}
      </div>
      <div ref={box} className="sim-chart-box" onPointerMove={onMove} onPointerLeave={() => setHover(null)}>
        {result && <RunBand result={result} view={view} at={at} x={x} w={w} width={width} across={across} xAxis={xAxis} t={t} />}
        <ProfileBand profile={profile} at={at} heightAt={heightAt} x={x} w={w} width={width} across={across} xAxis={xAxis} t={t} language={language} />
      </div>
    </div>
  )
}

/** The lines of the run in their words: the speed's three, the time's two. */
function RunLegend({ view, t }) {
  const lines = view === 'speed' ? ['mrsp', 'base', 'final'] : ['base', 'final']
  return (
    <div className="sim-legend">
      {lines.map(k => <span key={k} className="sim-legend-item"><svg width="22" height="8" aria-hidden="true"><line x1="0" y1="4" x2="22" y2="4" className={`sim-line-${k}`} /></svg>{t(`chart_${k}`)}</span>)}
    </div>
  )
}

/** The speed or the time over the distance. */
function RunBand({ result, view, at, x, w, width, across, xAxis, t }) {
  const height = RUN_HEIGHT
  const h = height - PAD.top - PAD.bottom
  const final = result.final
  let yMax, y, lines
  if (view === 'speed') {
    const vmax = Math.max(...result.mrsp.map(r => r.speed), ...final.speeds, ...result.base.speeds) * 3.6
    yMax = Math.ceil(vmax / 20) * 20 || 20
    y = (v) => PAD.top + h - (v / yMax) * h
    const line = (rep) => rep.positions.map((p, i) => `${x(p).toFixed(1)},${y(rep.speeds[i] * 3.6).toFixed(1)}`).join(' ')
    const mrsp = result.mrsp.flatMap(r => [`${x(r.begin).toFixed(1)},${y(r.speed * 3.6).toFixed(1)}`, `${x(r.end).toFixed(1)},${y(r.speed * 3.6).toFixed(1)}`]).join(' ')
    lines = [{ key: 'mrsp', points: mrsp }, { key: 'base', points: line(result.base) }, { key: 'final', points: line(final) }]
  } else {
    yMax = Math.max(final.total_time, 1)
    y = (s) => PAD.top + h - (s / yMax) * h
    const line = (rep) => rep.positions.map((p, i) => `${x(p).toFixed(1)},${y(rep.times[i]).toFixed(1)}`).join(' ')
    lines = [{ key: 'base', points: line(result.base) }, { key: 'final', points: line(final) }]
  }
  return (
    <svg width={width} height={height} role="img" aria-label={t(view === 'speed' ? 'chart_speed' : 'chart_time')}>
      {ticks(0, yMax, 5).map(v => (
        <g key={`y${v}`}>
          <line x1={PAD.left} x2={PAD.left + w} y1={y(v)} y2={y(v)} className="sim-grid" />
          <text x={PAD.left - 6} y={y(v) + 4} textAnchor="end" className="sim-axis">{view === 'speed' ? v : formatTime(v)}</text>
        </g>
      ))}
      {across(h)}
      {xAxis(height, h)}
      <text x={4} y={PAD.top - 2} className="sim-axis">{view === 'speed' ? 'km/h' : t('time_axis')}</text>
      {lines.map(l => <polyline key={l.key} points={l.points} className={`sim-line-${l.key}`} fill="none" />)}
      {at && <circle cx={x(at.position)} cy={view === 'speed' ? y(at.speed * 3.6) : y(at.time)} r="5" className="sim-at" />}
    </svg>
  )
}

/** The height along the stretch, filled down to the band's foot; a gap where a track has no heights. */
function ProfileBand({ profile, at, heightAt, x, w, width, across, xAxis, t, language }) {
  const height = PROFILE_HEIGHT
  const h = height - PAD.top - PAD.bottom
  const known = profile.filter(p => p.height != null).map(p => p.height)
  const lo = known.length ? Math.min(...known) : 0, hi = known.length ? Math.max(...known) : 1
  // At least 10 m of height shown, so a flat stretch does not look steep.
  const room = Math.max(hi - lo, 10)
  const [yMin, yMax] = [lo - (room - (hi - lo)) / 2 - room * 0.1, hi + (room - (hi - lo)) / 2 + room * 0.1]
  const y = (z) => PAD.top + h - ((z - yMin) / (yMax - yMin)) * h
  const parts = pieces(profile)
  const gaps = []
  for (let i = 1; i < profile.length; i++) if (profile[i].height == null || profile[i - 1].height == null) gaps.push([profile[i - 1].position, profile[i].position])
  const atHeight = at ? heightAt(at.position) : null
  return (
    <svg width={width} height={height} role="img" aria-label={t('chart_profile')}>
      {ticks(yMin, yMax, 3).map(v => (
        <g key={`y${v}`}>
          <line x1={PAD.left} x2={PAD.left + w} y1={y(v)} y2={y(v)} className="sim-grid" />
          <text x={PAD.left - 6} y={y(v) + 4} textAnchor="end" className="sim-axis">{v.toLocaleString(language)}</text>
        </g>
      ))}
      {gaps.map(([a, b], i) => <rect key={`g${i}`} x={x(a)} y={PAD.top} width={Math.max(0, x(b) - x(a))} height={h} className="sim-profile-gap"><title>{t('chart_no_heights')}</title></rect>)}
      {parts.map((part, i) => {
        const line = part.map(p => `${x(p.position).toFixed(1)},${y(p.height).toFixed(1)}`).join(' ')
        const foot = `${x(part.at(-1).position).toFixed(1)},${PAD.top + h} ${x(part[0].position).toFixed(1)},${PAD.top + h}`
        return (
          <g key={`h${i}`}>
            <polygon points={`${line} ${foot}`} className="sim-profile-fill" />
            <polyline points={line} className="sim-profile-line" fill="none" />
          </g>
        )
      })}
      {across(h)}
      {xAxis(height, h)}
      <text x={4} y={PAD.top - 2} className="sim-axis">{t('chart_height_axis')}</text>
      {!known.length && <text x={PAD.left + w / 2} y={PAD.top + h / 2} textAnchor="middle" className="sim-axis">{t('chart_no_heights')}</text>}
      {at && atHeight != null && <circle cx={x(at.position)} cy={y(atHeight)} r="4" className="sim-at" />}
    </svg>
  )
}
