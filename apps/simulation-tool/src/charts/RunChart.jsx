import { useEffect, useRef, useState } from 'react'
import { Tabs } from '@rail-across/ui'
import { formatTime } from '../format.js'

/**
 * The run over the path: the speed over the distance — the speed allowed
 * (MRSP), the fastest run and the run with its margin and times — or the
 * time over the distance; the stops marked, and the moment played.
 */

const PAD = { left: 52, right: 16, top: 14, bottom: 30 }

/** Round steps for an axis up to `max`, about `n` of them. */
function ticks(max, n) {
  if (!(max > 0)) return [0]
  const raw = max / n
  const mag = 10 ** Math.floor(Math.log10(raw))
  const step = [1, 2, 2.5, 5, 10].map(f => f * mag).find(s => s >= raw)
  const out = []
  for (let v = 0; v <= max + 1e-9; v += step) out.push(Math.round(v * 1e6) / 1e6)
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

export default function RunChart({ run, at, t }) {
  const [view, setView] = useState('speed')
  const box = useRef(null)
  const width = useWidth(box)
  const height = 220
  const { result, offsets, labels } = run
  const length = run.request.path.length
  const final = result.final
  const w = width - PAD.left - PAD.right, h = height - PAD.top - PAD.bottom
  const x = (s) => PAD.left + (s / length) * w
  const km = length >= 2000

  let yMax, y, lines
  if (view === 'speed') {
    const vmax = Math.max(...result.mrsp.map(r => r.speed), ...final.speeds, ...result.base.speeds) * 3.6
    yMax = Math.ceil(vmax / 20) * 20 || 20
    y = (v) => PAD.top + h - (v / yMax) * h
    const line = (rep) => rep.positions.map((p, i) => `${x(p).toFixed(1)},${y(rep.speeds[i] * 3.6).toFixed(1)}`).join(' ')
    const mrsp = result.mrsp.flatMap(r => [`${x(r.begin).toFixed(1)},${y(r.speed * 3.6).toFixed(1)}`, `${x(r.end).toFixed(1)},${y(r.speed * 3.6).toFixed(1)}`]).join(' ')
    lines = [
      { key: 'mrsp', points: mrsp, className: 'sim-line-mrsp', label: t('chart_mrsp') },
      { key: 'base', points: line(result.base), className: 'sim-line-base', label: t('chart_base') },
      { key: 'final', points: line(final), className: 'sim-line-final', label: t('chart_final') },
    ]
  } else {
    yMax = Math.max(final.total_time, 1)
    y = (s) => PAD.top + h - (s / yMax) * h
    const line = (rep) => rep.positions.map((p, i) => `${x(p).toFixed(1)},${y(rep.times[i]).toFixed(1)}`).join(' ')
    lines = [
      { key: 'base', points: line(result.base), className: 'sim-line-base', label: t('chart_base') },
      { key: 'final', points: line(final), className: 'sim-line-final', label: t('chart_final') },
    ]
  }
  const yTicks = ticks(yMax, 5)
  const xTicks = ticks(km ? length / 1000 : length, Math.max(2, Math.floor(w / 90))).map(v => (km ? v * 1000 : v))

  return (
    <div className="sim-chart">
      <div className="sim-chart-head">
        <Tabs value={view} onChange={setView} options={[{ value: 'speed', label: t('chart_speed') }, { value: 'time', label: t('chart_time') }]} label={t('chart')} />
        <div className="sim-legend">
          {lines.map(l => <span key={l.key} className="sim-legend-item"><svg width="22" height="8" aria-hidden="true"><line x1="0" y1="4" x2="22" y2="4" className={l.className} /></svg>{l.label}</span>)}
        </div>
      </div>
      <div ref={box} className="sim-chart-box">
        <svg width={width} height={height} role="img" aria-label={t(view === 'speed' ? 'chart_speed' : 'chart_time')}>
          {yTicks.map(v => (
            <g key={`y${v}`}>
              <line x1={PAD.left} x2={PAD.left + w} y1={y(v)} y2={y(v)} className="sim-grid" />
              <text x={PAD.left - 6} y={y(v) + 4} textAnchor="end" className="sim-axis">{view === 'speed' ? v : formatTime(v)}</text>
            </g>
          ))}
          {xTicks.map(v => (
            <g key={`x${v}`}>
              <line x1={x(v)} x2={x(v)} y1={PAD.top} y2={PAD.top + h} className="sim-grid" />
              <text x={x(v)} y={PAD.top + h + 16} textAnchor="middle" className="sim-axis">{km ? `${(v / 1000).toLocaleString()}` : Math.round(v)}</text>
            </g>
          ))}
          <text x={PAD.left + w} y={height - 2} textAnchor="end" className="sim-axis">{km ? 'km' : 'm'}</text>
          <text x={4} y={PAD.top - 2} className="sim-axis">{view === 'speed' ? 'km/h' : t('time_axis')}</text>
          {offsets.map((s, i) => (
            <g key={`p${i}`}>
              <line x1={x(s)} x2={x(s)} y1={PAD.top} y2={PAD.top + h} className="sim-stop" />
              <title>{labels[i]}</title>
            </g>
          ))}
          {lines.map(l => <polyline key={l.key} points={l.points} className={l.className} fill="none" />)}
          {at && <circle cx={x(at.position)} cy={view === 'speed' ? y(at.speed * 3.6) : y(at.time)} r="5" className="sim-at" />}
        </svg>
      </div>
    </div>
  )
}
