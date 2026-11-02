import { useMemo, useState } from 'react'
import { Badge, Button, GroupedList, Message, Option, Options, Section, SelectField, TextField } from '@rail-across/ui'
import { designationOf as signalDesignation } from '../model/domain.js'
import { searchItineraries, withItinerary } from '../model/itineraries.js'
import { stoppingPlaceLabel } from '../model/stoppingPlaces.js'
import { stationName } from '../model/stations.js'
import { RouteIcon } from '../icons.jsx'
import { EditorHead, ObjectFindings } from './common.jsx'

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

/**
 * Searching the itineraries between two stopping places: the ways the net has
 * from the one to the other, each with the train routes it takes — of the
 * stations whose signals start them, none on the open line —, those there are
 * already and those it would create; one is made an itinerary, with the
 * routes it lacks, and opens in its editor.
 */
export function ItinerarySearch({ domain, net, core, ops, select, t }) {
  const places = domain.collections.stoppingPlaces
  const options = places.map(h => ({ value: h.id, label: stoppingPlaceLabel(core, h, t) })).sort(byLabel)
  const [start, setStart] = useState(options[0]?.value ?? null)
  const [target, setTarget] = useState(options[1]?.value ?? null)
  const [asked, setAsked] = useState(null)
  const from = places.find(h => h.id === asked?.start), to = places.find(h => h.id === asked?.target)
  // Searched anew with every edit, so what was made since says so.
  const found = useMemo(() => (from && to ? searchItineraries(net, domain, from, to) : null), [net, domain, from, to])
  const signals = new Map(domain.collections.signals.map(s => [s.id, s]))
  const points = new Map((core.collections.operationalPoints ?? []).map(p => [p.id, p]))
  const switchName = (id) => { const sw = net.switches.find(x => x.switchId === id); return sw?.name ?? sw?.label ?? id.slice(0, 8) }
  const define = (r, i) => {
    const via = found.length > 1 && r.switches.length ? ` · ${t('via_switches', { list: r.switches.map(switchName).join(', ') })}` : ''
    const name = `${stoppingPlaceLabel(core, from, t)} → ${stoppingPlaceLabel(core, to, t)}`
    const { state, id } = withItinerary(domain, r, from, to, `${name}${via || (found.length > 1 ? ` (${i + 1})` : '')}`)
    ops.replaceAll(state)
    select({ collection: 'itineraries', id })
  }
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('itinerary_start')} value={start} options={options} onChange={setStart} />
      <SelectField compact wide label={t('itinerary_target')} value={target} options={options} onChange={setTarget} />
    </div>
    <div className="ui-row">
      <Button variant="primary" disabled={!start || !target || start === target} onClick={() => setAsked({ start, target })}>{t('itinerary_search')}</Button>
    </div>
    {found && !found.length && <p className="ui-list-empty">{t('itinerary_none_found')}</p>}
    {found?.map((r, i) => (
      <div key={r.way.trackIds.join(',')} className="tool-way">
        <div className="tool-way-head">
          <span className="ui-grow">{t('way_n', { n: i + 1 })} · {Math.round(r.way.length)} m</span>
          {r.existingId && <Badge>{t('itinerary_defined')}</Badge>}
        </div>
        {r.switches.length > 0 && <span className="ui-muted">{t('via_switches', { list: r.switches.map(switchName).join(', ') })}</span>}
        <ul>
          {r.routes.map(x => (
            <li key={`${x.startSignalId}-${x.targetSignalId}`}>
              {signalDesignation(signals.get(x.startSignalId), t)} → {signalDesignation(signals.get(x.targetSignalId), t)}
              <span className="ui-muted"> · {stationName(points.get(x.stationId))} · {t(x.existingId ? 'route_there' : 'route_new')}</span>
            </li>
          ))}
        </ul>
        {r.problems.map(p => <Message key={p} tone="warn">{t(`itinerary_problem_${p}`)}</Message>)}
        <div className="ui-row">
          {r.existingId
            ? <Button size="small" onClick={() => select({ collection: 'itineraries', id: r.existingId })}>{t('edit_it')}</Button>
            : <Button size="small" variant="primary" disabled={r.problems.length > 0} onClick={() => define(r, i)}>
              {t('itinerary_define', { n: r.routes.filter(x => !x.existingId).length })}
            </Button>}
        </div>
      </div>
    ))}
  </>
}

/** The itineraries, under the station they start at. */
export function ItineraryList({ domain, core, findings, select, t }) {
  const places = new Map(domain.collections.stoppingPlaces.map(h => [h.id, h]))
  const points = new Map((core.collections.operationalPoints ?? []).map(p => [p.id, p]))
  const bad = new Set(findings.errors.filter(f => f.collection === 'itineraries').map(f => f.id))
  const groups = new Map()
  for (const it of domain.collections.itineraries) {
    const pointId = places.get(it.startStoppingPlaceId)?.operationalPointId ?? null
    if (!groups.has(pointId)) groups.set(pointId, [])
    const to = places.get(it.targetStoppingPlaceId)
    groups.get(pointId).push({
      id: it.id, label: it.name ?? t('unnamed'),
      sub: [to ? stoppingPlaceLabel(core, to, t) : null, t('n_routes', { n: it.routeIds.length })].filter(Boolean).join(' · '),
      end: bad.has(it.id) ? <Badge tone="danger">!</Badge> : null,
    })
  }
  const list = [...groups].map(([pointId, items]) => ({
    key: pointId ?? 'open', title: pointId ? stationName(points.get(pointId)) : t('open_line'), items: items.sort(byLabel),
  })).sort((a, b) => a.title.localeCompare(b.title, 'de', { numeric: true }))
  return (
    <GroupedList label={t('itineraries')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('empty_list')}
      groups={list} onSelect={(id) => select({ collection: 'itineraries', id })} />
  )
}

/** An itinerary: its name, its two stopping places, the routes it takes in running order. */
export function ItineraryEditor({ itinerary, domain, core, findings, ops, select, readOnly, t }) {
  const places = new Map(domain.collections.stoppingPlaces.map(h => [h.id, h]))
  const signals = new Map(domain.collections.signals.map(s => [s.id, s]))
  const routes = new Map(domain.collections.routes.map(r => [r.id, r]))
  const place = (id) => (places.get(id) ? stoppingPlaceLabel(core, places.get(id), t) : t('unnamed'))
  return (
    <div className="tool-editor">
      <EditorHead title={itinerary.name ?? t('unnamed')} readOnly={readOnly} t={t} onDelete={() => ops.remove('itineraries', itinerary.id)} />
      <ObjectFindings findings={findings} collection="itineraries" id={itinerary.id} t={t} />
      <div className="ui-fields">
        <TextField compact wide label={t('name')} value={itinerary.name ?? ''} disabled={readOnly} onChange={(v) => ops.update('itineraries', { ...itinerary, name: v || undefined })} />
      </div>
      <dl className="tool-facts">
        <dt>{t('itinerary_start')}</dt><dd>{place(itinerary.startStoppingPlaceId)}</dd>
        <dt>{t('itinerary_target')}</dt><dd>{place(itinerary.targetStoppingPlaceId)}</dd>
      </dl>
      <Section title={t('section_routes')}>
        <p className="ui-muted">{t('itinerary_routes_hint')}</p>
        <Options>
          {itinerary.routeIds.map(id => {
            const r = routes.get(id)
            return (
              <Option key={id} icon={<RouteIcon />} onClick={() => r && select({ collection: 'routes', id })}>
                {r ? `${signalDesignation(signals.get(r.startSignalId), t)} → ${signalDesignation(signals.get(r.targetSignalId), t)}` : t('unnamed')}
              </Option>
            )
          })}
        </Options>
      </Section>
    </div>
  )
}
