import { useEffect, useMemo, useState } from 'react'
import { GroupedList, Section } from '@rail-across/ui'
import { labelOf } from '../model/domain.js'
import { speedRanges } from '../model/speedRanges.js'
import { inDirection } from '../model/directions.js'

/** A range as it is named: its signal and its speed — 21-A · 60 km/h. */
const rangeName = (r, t) => `${labelOf('signals', r.signal, t)} · ${r.speed} km/h`

/**
 * The speed ranges (speedRanges) as the track sections are shown: listed in
 * their colours, under the start signals' and the Lf 7's, and drawn on the
 * plan each in its colour, its name on it in that colour. A click on one
 * draws it alone, brings it into view and names the routes it holds in; a
 * second click shows them all again. Only those of `direction` (directions.js).
 */
export default function SpeedRanges({ domain, net, direction = 'both', routeName, select, setSketch, showOnPlan, t }) {
  const all = useMemo(() => speedRanges(domain, net), [domain, net])
  const ranges = useMemo(() => all.filter(r => inDirection(net, r.signal.at, direction)), [all, net, direction])
  const [chosenId, setChosenId] = useState(null)
  const chosen = ranges.find(r => r.id === chosenId) ?? null
  useEffect(() => {
    const shown = chosen ? [chosen] : ranges
    const stretches = shown.flatMap(r => r.parts.map(p => ({ ...p, color: r.color, id: r.signal.id })))
    const labels = shown.map(r => {
      const p = [...r.parts].sort((a, b) => (b.to - b.from) - (a.to - a.from))[0]
      return p ? { ...p, text: rangeName(r, t), color: r.color } : null
    }).filter(Boolean)
    setSketch?.({ stretches, labels, ids: shown.map(r => r.signal.id) })
    return () => setSketch?.(null)
  }, [ranges, chosen, setSketch, t])

  if (!ranges.length) return <p className="ui-muted">{t('speed_ranges_none')}</p>
  const item = (r) => ({
    id: r.id, label: rangeName(r, t), color: r.color,
    sub: t('speed_range_sub', { routes: r.routeIds.length, length: Math.round(r.length) }),
  })
  const byName = (a, b) => a.label.localeCompare(b.label, 'de', { numeric: true })
  const groups = [
    { key: 'start', title: t('speed_ranges_start'), items: ranges.filter(r => !r.board).map(item).sort(byName) },
    { key: 'board', title: t('speed_ranges_board'), items: ranges.filter(r => r.board).map(item).sort(byName) },
  ]
  const choose = (id) => {
    const r = id === chosenId ? null : ranges.find(x => x.id === id)
    setChosenId(r?.id ?? null)
    if (r) showOnPlan?.(r.parts.flatMap(p => [p.from, p.to].map(station => ({ anchor: 'station', trackId: p.trackId, station }))))
  }
  const routes = chosen ? domain.collections.routes.filter(r => chosen.routeIds.includes(r.id)) : []
  return <>
    <GroupedList label={t('speed_ranges')} open groups={groups} selected={chosenId} searchLabel={t('search_objects')} noneFound={t('search_none')}
      empty={t('speed_ranges_none')} onSelect={choose} />
    {chosen && (
      <Section title={t('speed_range_routes')}>
        <ul className="tool-speed-sections">
          {routes.map(r => (
            <li key={r.id}><button type="button" className="ui-link" onClick={() => select({ collection: 'routes', id: r.id })}>{routeName(r)}</button></li>
          ))}
        </ul>
      </Section>
    )}
  </>
}
