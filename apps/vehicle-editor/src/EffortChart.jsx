import { resistanceAt } from '@rail-across/vehicles'

const W = 560, H = 280
const PAD = { left: 48, right: 14, top: 24, bottom: 34 }

/** A round step for an axis up to `max`, about `n` of them. */
function niceStep(max, n = 6) {
  const raw = max / n
  const p = 10 ** Math.floor(Math.log10(raw || 1))
  return [1, 2, 2.5, 5, 10].map(f => f * p).find(s => s >= raw) ?? 10 * p
}

/**
 * The tractive effort over speed of a mode: the curve chosen and the mode's
 * others, the resistance to movement on the level, the top speed — in km/h
 * and kN, as OSRD's editor draws it.
 */
export default function EffortChart({ t, vehicle, curves, chosen }) {
  const vmax = vehicle.max_speed * 3.6
  const lines = curves.map(c => ({
    ...c,
    points: (c.curve?.speeds ?? []).map((s, i) => [s * 3.6, (c.curve.max_efforts[i] ?? NaN) / 1000]).filter(([x, y]) => Number.isFinite(x) && Number.isFinite(y)),
  }))
  const xTop = Math.max(vmax, ...lines.flatMap(l => l.points.map(p => p[0])), 10)
  const xStep = niceStep(xTop)
  const xMax = Math.ceil(xTop / xStep) * xStep
  const resistance = Array.from({ length: 41 }, (_, i) => { const x = (xMax * i) / 40; return [x, resistanceAt(vehicle, x / 3.6) / 1000] })
  const yTop = Math.max(...lines.flatMap(l => l.points.map(p => p[1])), ...resistance.filter(p => p[0] <= vmax).map(p => p[1]), 1)
  const yStep = niceStep(yTop, 5)
  const yMax = Math.ceil(yTop / yStep) * yStep
  const sx = (x) => PAD.left + (x / xMax) * (W - PAD.left - PAD.right)
  const sy = (y) => H - PAD.bottom - (Math.min(y, yMax * 1.05) / yMax) * (H - PAD.top - PAD.bottom)
  const path = (pts) => pts.map(([x, y], i) => `${i ? 'L' : 'M'}${sx(x).toFixed(1)},${sy(y).toFixed(1)}`).join('')
  const ticks = (max, step) => Array.from({ length: Math.round(max / step) + 1 }, (_, i) => i * step)
  const current = lines.find(l => l.index === chosen)

  return (
    <figure className="veh-chart">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={t('effort_chart')}>
        {ticks(yMax, yStep).map(y => (
          <g key={`y${y}`}>
            <line className="veh-grid-line" x1={PAD.left} x2={W - PAD.right} y1={sy(y)} y2={sy(y)} />
            <text className="veh-axis" x={PAD.left - 6} y={sy(y) + 4} textAnchor="end">{y}</text>
          </g>
        ))}
        {ticks(xMax, xStep).map(x => (
          <text key={`x${x}`} className="veh-axis" x={sx(x)} y={H - PAD.bottom + 16} textAnchor="middle">{x}</text>
        ))}
        <text className="veh-axis" x={W - PAD.right} y={H - 4} textAnchor="end">km/h</text>
        <text className="veh-axis" x={PAD.left - 6} y={11} textAnchor="end">kN</text>
        <line className="veh-line-top" x1={sx(vmax)} x2={sx(vmax)} y1={PAD.top} y2={H - PAD.bottom} />
        <path className="veh-line-resistance" d={path(resistance)} fill="none" />
        {lines.filter(l => l !== current).map(l => <path key={l.index} className="veh-line-other" d={path(l.points)} fill="none" />)}
        {current && <path className="veh-line-curve" d={path(current.points)} fill="none" />}
        {current?.points.map(([x, y], i) => <circle key={i} className="veh-point" cx={sx(x)} cy={sy(y)} r={2.6} />)}
      </svg>
      <figcaption className="veh-legend">
        <span className="veh-legend-item"><svg width="18" height="6"><line className="veh-line-curve" x1="0" x2="18" y1="3" y2="3" /></svg>{t('effort_curve')}</span>
        {lines.length > 1 && <span className="veh-legend-item"><svg width="18" height="6"><line className="veh-line-other" x1="0" x2="18" y1="3" y2="3" /></svg>{t('effort_others')}</span>}
        <span className="veh-legend-item"><svg width="18" height="6"><line className="veh-line-resistance" x1="0" x2="18" y1="3" y2="3" /></svg>{t('resistance_level')}</span>
        <span className="veh-legend-item"><svg width="18" height="6"><line className="veh-line-top" x1="0" x2="18" y1="3" y2="3" /></svg>{t('max_speed_line')}</span>
      </figcaption>
    </figure>
  )
}
