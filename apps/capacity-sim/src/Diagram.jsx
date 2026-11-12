import { categoryColor, formatClock } from '@rail-across/timetables'

/**
 * A stretch's blocking time stairs: the stretch across, from its first place
 * to its other, time downwards; each block a box from where it begins to
 * where it ends on the stretch, from when it is held to when it is free, in
 * its train's colour. `compressed` draws each train moved as UIC 406
 * compresses it (the analysis's `shift`).
 */
const W = 1000, LEFT = 60, RIGHT = 12, TOP = 20

export default function Diagram({ analysis, compressed, fromName, toName }) {
  const shifts = new Map(analysis.trains.map(t => [t.id, compressed ? t.shift : 0]))
  const boxes = analysis.input.flatMap(train => train.blocks
    .filter(b => b.span)
    .map((b, i) => ({ key: `${train.id}-${i}`, id: train.id, kind: train.kind, x0: b.span[0], x1: b.span[1], t0: b.begin - shifts.get(train.id), t1: b.end - shifts.get(train.id) })))
  if (!boxes.length) return null
  const t0 = Math.min(...boxes.map(b => b.t0)), t1 = Math.max(...boxes.map(b => b.t1))
  const height = Math.max(240, Math.min(900, (t1 - t0) / 4))
  const H = TOP + height + 10
  const x = (f) => LEFT + f * (W - LEFT - RIGHT)
  const y = (s) => TOP + (s - t0) / (t1 - t0 || 1) * height
  const ticks = []
  const step = (t1 - t0) > 4 * 3600 ? 3600 : (t1 - t0) > 3600 ? 900 : 300
  for (let s = Math.ceil(t0 / step) * step; s <= t1; s += step) ticks.push(s)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} role="img">
      <text x={LEFT} y={12}>{fromName}</text>
      <text x={W - RIGHT} y={12} textAnchor="end">{toName}</text>
      {ticks.map(s => (
        <g key={s}>
          <line x1={LEFT} x2={W - RIGHT} y1={y(s)} y2={y(s)} stroke="currentColor" strokeOpacity="0.08" />
          <text x={LEFT - 6} y={y(s) + 4} textAnchor="end">{formatClock(s)}</text>
        </g>
      ))}
      {boxes.map(b => (
        <rect key={b.key} x={x(b.x0)} y={y(b.t0)} width={Math.max(1, x(b.x1) - x(b.x0))} height={Math.max(1, y(b.t1) - y(b.t0))}
          fill={categoryColor(b.kind)} fillOpacity="0.28" stroke={categoryColor(b.kind)} strokeWidth="1">
          <title>{`${b.id} · ${formatClock(b.t0, { seconds: true })}–${formatClock(b.t1, { seconds: true })}`}</title>
        </rect>
      ))}
    </svg>
  )
}
