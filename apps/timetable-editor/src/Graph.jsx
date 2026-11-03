import { useEffect, useRef, useState } from 'react'
import { categoryColor, formatClock, trainTitle } from '@rail-across/timetables'
import { axisOf, blocksOn, linesOn } from './rows.js'

/**
 * The graphical timetable (Bildfahrplan): the stations along the way of one
 * train across, at their distances, the time downwards, and every train of
 * the timetable that runs along that way as a line — sloping where it runs,
 * straight down where it stands; under it, with `view.blocks`, the blocks
 * it holds as they are held over time (Sperrzeitentreppe), red where
 * another holds one at the same time. A line's train is chosen with a click.
 * `view` { from, to, scale, blocks } — the hours shown (all the trains'
 * without), pixels per minute, whether blocking times are drawn.
 */
const LEFT = 56, RIGHT = 80, TOP = 120, BOTTOM = 24

export default function Graph({ t, trains, runs, axisTrain, view, selected, onSelect }) {
  const box = useRef(null)
  const [width, setWidth] = useState(900)
  useEffect(() => {
    if (!box.current || typeof ResizeObserver === 'undefined') return undefined
    const watch = new ResizeObserver(([e]) => setWidth(Math.max(480, Math.floor(e.contentRect.width))))
    watch.observe(box.current)
    return () => watch.disconnect()
  }, [])

  const axisRun = axisTrain ? runs.get(axisTrain.id) : null
  if (!axisRun?.rows) {
    return <div className="tt-scroll" ref={box}><div className="tt-page tt-empty"><p className="ui-muted">{t(trains.length ? 'graph_no_axis' : 'trains_none_lead')}</p></div></div>
  }
  const axis = axisOf(axisRun.rows)
  const drawn = trains.map(tr => {
    const run = runs.get(tr.id)
    const bad = new Set((run?.conflicts ?? []).map(c => c.block))
    const held = view.blocks ? blocksOn(axis, run?.rows, run?.blocks).map(b => ({ ...b, conflict: bad.has(b.block) })) : []
    return { train: tr, lines: linesOn(axis, run?.rows), held }
  }).filter(d => d.lines.length)
  const times = drawn.flatMap(d => d.lines.flat().map(p => p.t))
  const from = view.from ?? Math.floor(Math.min(...times) / 3600) * 3600
  const to = view.to ?? Math.ceil(Math.max(...times) / 3600) * 3600
  const span = Math.max(to - from, 3600)
  const height = TOP + BOTTOM + span / 60 * view.scale
  const total = axis[axis.length - 1].at || 1
  const xAt = (at) => LEFT + (at / total) * (width - LEFT - RIGHT)
  const x = (i) => xAt(axis[i].at)
  const y = (s) => TOP + (s - from) / 60 * view.scale
  const ticks = []
  const minor = view.scale >= 6 ? 300 : 600
  for (let s = Math.ceil(from / minor) * minor; s <= from + span; s += minor) ticks.push(s)

  return (
    <div className="tt-scroll" ref={box}>
      <svg className="tt-graph" width={width} height={height} role="img" aria-label={t('section_graph')}>
        <defs><clipPath id="tt-graph-clip"><rect x={0} y={TOP} width={width} height={height - TOP - BOTTOM} /></clipPath></defs>
        {ticks.map(s => (
          <g key={s}>
            <line className={s % 3600 === 0 ? 'tt-grid-hour' : 'tt-grid'} x1={LEFT} x2={width - RIGHT} y1={y(s)} y2={y(s)} />
            <text className={s % 3600 === 0 ? 'tt-axis tt-axis-hour' : 'tt-axis'} x={LEFT - 6} y={y(s) + 4} textAnchor="end">{formatClock(s)}</text>
          </g>
        ))}
        {axis.map((s, i) => (
          <g key={`${s.key}-${i}`}>
            <line className="tt-station-line" x1={x(i)} x2={x(i)} y1={TOP - 6} y2={height - BOTTOM} />
            <text className="tt-station-name" transform={`translate(${x(i) + 4} ${TOP - 10}) rotate(-45)`}>{s.name}</text>
          </g>
        ))}
        <g clipPath="url(#tt-graph-clip)">
          {drawn.map(({ train, held }) => (
            <g key={`b-${train.id}`} className="tt-blocks">
              {held.map((b, k) => (
                <rect key={k} className={b.conflict ? 'tt-block tt-block-conflict' : 'tt-block'} x={xAt(b.a0)} y={y(b.begin)}
                  width={Math.max(2, xAt(b.a1) - xAt(b.a0))} height={Math.max(1, y(b.end) - y(b.begin))}
                  fill={b.conflict ? undefined : categoryColor(train.category)} />
              ))}
            </g>
          ))}
          {drawn.map(({ train, lines }) => {
            const color = categoryColor(train.category)
            const chosen = train.id === selected
            return (
              <g key={train.id} className={`tt-train${chosen ? ' tt-train-chosen' : ''}`} onClick={() => onSelect(train.id)}>
                <title>{trainTitle(train) || t('train_unnamed')}</title>
                {lines.map((line, k) => {
                  const d = line.map((p, j) => `${j ? 'L' : 'M'}${x(p.i).toFixed(1)} ${y(p.t).toFixed(1)}`).join(' ')
                  const head = line[0], next = line[1]
                  const angle = Math.atan2(y(next.t) - y(head.t), x(next.i) - x(head.i)) * 180 / Math.PI
                  return (
                    <g key={k}>
                      <path className="tt-train-hit" d={d} />
                      <path className="tt-train-line" d={d} stroke={color} strokeWidth={chosen ? 3 : 1.8} />
                      {k === 0 && train.number && (
                        <text className="tt-train-label" fill={color} transform={`translate(${x(head.i)} ${y(head.t)}) rotate(${Math.abs(angle) > 90 ? angle + 180 : angle}) translate(${Math.abs(angle) > 90 ? -6 : 6} -4)`}
                          textAnchor={Math.abs(angle) > 90 ? 'end' : 'start'}>{train.number}</text>
                      )}
                    </g>
                  )
                })}
              </g>
            )
          })}
        </g>
      </svg>
    </div>
  )
}
