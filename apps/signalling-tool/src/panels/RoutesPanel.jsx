import { useEffect, useMemo, useState } from 'react'
import {
  Badge, Button, Checkbox, GroupedList, NumberField, Option, Options, OptionsDivider, OptionsHeading, PlusIcon, Section, SelectField, TextField,
} from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { signallingSpeedOf } from '@rail-across/core/signalling'
import {
  added, designationOf as signalDesignation, newId, labelOf, logicalId, shownSpeed, withRouteSpeed,
} from '../model/domain.js'
import { routeElementSpeed, routeSpeedSections, speedSignalSpeed, withElementSpeeds } from '../model/routeSpeeds.js'
import { measuredOverlap, overlapSpeed, withMeasuredOverlap } from '../model/overlaps.js'
import { findRun, runAhead, switchesOnRun } from '../model/runs.js'
import { SHUNTING_TYPE, TRAIN_TYPE, optionsOf } from '../labels.js'
import { ItineraryIcon, OverlapIcon, RouteIcon } from '../icons.jsx'
import { stationName } from '../model/stations.js'
import { ItineraryEditor, ItineraryList, ItinerarySearch } from './Itineraries.jsx'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'
import { aspectText, pickingSignal, signalOptions as signalChoices, trackName } from '../model/places.js'
import { ElementLink } from './speedReach.jsx'
import { isMainSignal } from '../model/itineraries.js'
import { nextSignals, routeSections, routeStretches, runStretches, wayPieces } from '../model/routeSearch.js'

/** The kinds of route, each with the type it starts with. */
const KINDS = { train: { trainType: 'ZZ' }, shunting: { shuntingType: 'RR' }, middle: { middleType: 'ZM' } }

/**
 * Routes and their overlaps, with a menu as the Layout-Tool's panels: a new
 * route from a start signal to a target, over the shortest run the net has
 * the way the two act; a new overlap ahead of a signal; the routes under
 * their kinds and the overlaps, each opened in its editor. A route shows the
 * aspects of its start signal that are chosen, and names the overlaps at its
 * target. The speed it permits is its start signal's to signal (Bearbeiten):
 * here it is shown, and an overlap is measured from it — its length after the
 * speed and the gradient over it, which the heights of the tracks give. A new
 * overlap runs its length ahead of its signal, straight on at every switch;
 * its danger point lies at its end.
 *
 * Routes are found from a signal too: to every next main signal ahead of it,
 * one route per way, with the speed they are run at; a route names the track
 * sections it runs through, as they stand.
 *
 * Routes are a station's: the station of their start signal, under which
 * they are listed; the open line has none, the block has its signals. The
 * itineraries from one stopping place to another are searched here over the
 * net, and made with the routes they take.
 */
export default function RoutesPanel({
  domain, net, core, findings, extents, selection, select, ops, mode, setMode, setSketch, showOnPlan, readOnly, t, language, setNotice,
}) {
  const [page, setPage] = useState('menu')
  const { routes, overlaps, signals, itineraries, stoppingPlaces } = domain.collections
  const sig = (id) => signals.find(s => s.id === id)
  const sigName = (id) => signalDesignation(sig(id), t)
  const routeName = (r) => r.name ?? `${sigName(r.startSignalId)} → ${sigName(r.targetSignalId)}`
  const bad = new Set(findings.errors.filter(f => f.collection === 'routes' || f.collection === 'overlaps').map(f => f.id))
  const chosenRoute = selection?.collection === 'routes' ? routes.find(r => r.id === selection.id) : null
  const chosenOverlap = selection?.collection === 'overlaps' ? overlaps.find(o => o.id === selection.id) : null
  const chosenItinerary = selection?.collection === 'itineraries' ? itineraries.find(i => i.id === selection.id) : null
  const back = { label: t('back'), onClick: () => setPage('menu') }
  const toPage = { label: t('back'), onClick: () => select(null) }

  const real = signals.filter(s => s.at?.anchor === 'station')
  const signalOptions = signalChoices(net, real, t)

  const newOverlap = (signalId, length = 100) => {
    const s = sig(signalId)
    if (!s) return
    const run = runAhead(net, s.at, length)
    const name = `D${overlaps.filter(o => o.signalId === signalId).length + 1}`
    setPage('overlaps')
    ops.add('overlaps', { id: newId(), name, signalId, path: { trackIds: run.trackIds }, end: { anchor: 'station', ...run.end }, length })
  }

  // Every train route signals what its elements allow up to its first Lf 7, every Lf 7 shows what lies ahead of it.
  const takeElementSpeeds = () => {
    const { domain: next, routes: r, boards } = withElementSpeeds(domain, net)
    if (r || boards) ops.replaceAll(next)
    setNotice?.({ tone: r || boards ? 'ok' : 'info', text: t('element_speeds_taken', { routes: r, boards }) })
  }

  if (chosenRoute) {
    return (
      <Panel title={t('object_routes')} back={toPage}>
        <RouteEditor key={chosenRoute.id} route={chosenRoute} name={routeName(chosenRoute)} domain={domain} net={net} findings={findings} extents={extents}
          signalOptions={signalOptions} readOnly={readOnly} t={t} language={language} select={select} setSketch={setSketch} showOnPlan={showOnPlan}
          onChange={(r) => ops.update('routes', r)} onDelete={() => ops.remove('routes', chosenRoute.id)} onNewOverlap={newOverlap}
          onSignal={(speed) => ops.replaceAll(withRouteSpeed(domain, chosenRoute, speed))}
          onMeasure={() => {
            const { domain: next, problem } = withMeasuredOverlap(domain, net, chosenRoute)
            if (next) ops.replaceAll(next)
            else setNotice?.({ tone: 'warn', text: t(`measure_no_${problem}`) })
          }} />
      </Panel>
    )
  }
  if (chosenItinerary) {
    return (
      <Panel title={t('object_itineraries')} back={toPage}>
        <ItineraryEditor key={chosenItinerary.id} itinerary={chosenItinerary} domain={domain} core={core} findings={findings} ops={ops} select={select} readOnly={readOnly} t={t} />
      </Panel>
    )
  }
  if (page === 'search') {
    return (
      <Panel title={t('itinerary_search_title')} intro={t('itinerary_search_intro')} back={back}>
        <ItinerarySearch domain={domain} net={net} core={core} ops={ops} select={select} mode={mode} setMode={setMode} setSketch={setSketch}
          showOnPlan={showOnPlan} t={t} />
      </Panel>
    )
  }
  if (page === 'itineraries') {
    return (
      <Panel title={t('itineraries')} intro={t('itineraries_intro')} back={back}>
        <ItineraryList domain={domain} core={core} findings={findings} select={select} t={t} />
      </Panel>
    )
  }
  if (chosenOverlap) {
    return (
      <Panel title={t('object_overlaps')} back={toPage}>
        <OverlapEditor key={chosenOverlap.id} overlap={chosenOverlap} domain={domain} net={net} findings={findings} extents={extents} setNotice={setNotice}
          signalOptions={signalOptions} readOnly={readOnly} t={t}
          onChange={(o) => ops.update('overlaps', o)} onDelete={() => ops.remove('overlaps', chosenOverlap.id)} />
      </Panel>
    )
  }

  if (page === 'new_route') {
    return (
      <Panel title={t('create_route')} intro={t('create_route_intro')} back={back}>
        <NewRoute signalOptions={signalOptions} t={t} language={language}
          onCreate={(r) => {
            const a = sig(r.startSignalId), b = sig(r.targetSignalId)
            const run = a && b ? findRun(net, a.at, b.at) : null
            const route = { ...r, id: newId(), path: { trackIds: run ?? [] } }
            setPage('routes')
            // It signals the speed its elements give.
            ops.replaceAll(withElementSpeeds(added(domain, 'routes', route), net, { routeIds: [route.id], boards: false }).domain)
            select({ collection: 'routes', id: route.id })
          }} />
      </Panel>
    )
  }
  if (page === 'find_routes') {
    return (
      <Panel title={t('find_routes')} intro={t('find_routes_intro')} back={back}>
        <FindRoutes domain={domain} net={net} extents={extents} ops={ops} setMode={setMode} setSketch={setSketch} t={t} />
      </Panel>
    )
  }
  if (page === 'new_overlap') {
    return (
      <Panel title={t('create_overlap')} intro={t('create_overlap_intro')} back={back}>
        <NewOverlap signalOptions={signalOptions} t={t} onCreate={newOverlap} />
      </Panel>
    )
  }

  if (page === 'routes') {
    const item = (r) => ({
      id: r.id, label: routeName(r),
      sub: [r.kind === 'train' ? null : t(`routes_${r.kind}`), r.trainType ?? r.shuntingType ?? r.middleType,
        signallingSpeedOf(domain, r) != null ? `${signallingSpeedOf(domain, r)} km/h` : null, r.rank ? `#${r.rank}` : null].filter(Boolean).join(' · '),
      end: bad.has(r.id) ? <Badge tone="danger">!</Badge> : null,
    })
    // Under the station of their start signal; those of a signal of the open line last.
    const points = new Map((core?.collections.operationalPoints ?? []).map(p => [p.id, p]))
    const byStation = new Map()
    for (const r of routes) {
      const key = sig(r.startSignalId)?.operationalPointId ?? null
      if (!byStation.has(key)) byStation.set(key, [])
      byStation.get(key).push(item(r))
    }
    const groups = [...byStation].map(([key, items]) => ({
      key: key ?? 'open', title: key ? stationName(points.get(key)) : t('routes_open_line'),
      items: items.sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true })),
    })).sort((a, b) => (a.key === 'open') - (b.key === 'open') || a.title.localeCompare(b.title, 'de', { numeric: true }))
    return (
      <Panel title={t('section_routes')} intro={t('routes_list_intro')} back={back}>
        <GroupedList label={t('section_routes')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('empty_list')}
          groups={groups} onSelect={(id) => select({ collection: 'routes', id })} />
      </Panel>
    )
  }
  if (page === 'overlaps') {
    return (
      <Panel title={t('overlaps')} intro={t('overlaps_list_intro')} back={back}>
        <GroupedList label={t('overlaps')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('empty_list')}
          groups={[{
            key: 'overlaps', title: t('overlaps'),
            items: overlaps.map(o => ({
              id: o.id, label: `${sigName(o.signalId)} ${o.name ?? ''}`.trim(), sub: `${o.length} m${o.speed ? ` · ${o.speed} km/h` : ''}`,
              end: bad.has(o.id) ? <Badge tone="danger">!</Badge> : null,
            })).sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true })),
          }]}
          onSelect={(id) => select({ collection: 'overlaps', id })} />
      </Panel>
    )
  }

  return (
    <Panel title={t('section_routes')} intro={t('section_routes_intro')}>
      {!readOnly && <>
        <Options>
          <OptionsHeading>{t('menu_create')}</OptionsHeading>
          <Option icon={<ItineraryIcon />} disabled={stoppingPlaces.length < 2} onClick={() => setPage('search')}>{t('itinerary_search_title')}</Option>
          <Option icon={<RouteIcon />} disabled={real.length < 2} onClick={() => setPage('find_routes')}>{t('find_routes')}</Option>
          <Option icon={<RouteIcon />} disabled={real.length < 2} onClick={() => setPage('new_route')}>{t('create_route')}</Option>
          <Option icon={<OverlapIcon />} disabled={!real.length} onClick={() => setPage('new_overlap')}>{t('create_overlap')}</Option>
        </Options>
        <OptionsDivider />
      </>}
      <Options>
        <OptionsHeading>{t('section_edit')}</OptionsHeading>
        <Option icon={<ItineraryIcon />} end={itineraries.length} onClick={() => setPage('itineraries')}>{t('itineraries')}</Option>
        <Option icon={<RouteIcon />} end={routes.length} onClick={() => setPage('routes')}>{t('section_routes')}</Option>
        <Option icon={<OverlapIcon />} end={overlaps.length} onClick={() => setPage('overlaps')}>{t('overlaps')}</Option>
        {!readOnly && <Option icon={<RouteIcon />} disabled={!routes.length} onClick={takeElementSpeeds}>{t('take_element_speeds')}</Option>}
      </Options>
    </Panel>
  )
}

/** A new route: its kind and type, its start and target signal. */
function NewRoute({ signalOptions, onCreate, t, language }) {
  const [kind, setKind] = useState('train')
  const [type, setType] = useState('ZZ')
  const [start, setStart] = useState(signalOptions[0]?.value ?? null)
  const [target, setTarget] = useState(signalOptions[1]?.value ?? null)
  const typeField = kind === 'train' ? 'trainType' : kind === 'shunting' ? 'shuntingType' : 'middleType'
  return <>
    <div className="ui-fields">
      <SelectField compact label={t('route_kind')} value={kind}
        options={Object.keys(KINDS).map(k => ({ value: k, label: t(k) }))}
        onChange={(v) => { setKind(v); setType(Object.values(KINDS[v])[0]) }} />
      {kind === 'train' && <SelectField compact label={t('train_type')} value={type} options={optionsOf(TRAIN_TYPE, language)} onChange={setType} />}
      {kind === 'shunting' && <SelectField compact label={t('shunting_type')} value={type} options={optionsOf(SHUNTING_TYPE, language)} onChange={setType} />}
      <SelectField compact wide label={t('start_signal')} value={start} options={signalOptions} onChange={setStart} />
      <SelectField compact wide label={t('target_signal')} value={target} options={signalOptions} onChange={setTarget} />
    </div>
    <div className="ui-row">
      <Button variant="primary" icon={<PlusIcon size={14} />} disabled={!start || !target || start === target}
        onClick={() => onCreate({ kind, [typeField]: type, startSignalId: start, targetSignalId: target })}>{t('create')}</Button>
    </div>
  </>
}

/** A new overlap: the signal it lies ahead of, its length. */
function NewOverlap({ signalOptions, onCreate, t }) {
  const [signalId, setSignalId] = useState(signalOptions[0]?.value ?? null)
  const [length, setLength] = useState(100)
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('at_signal')} value={signalId} options={signalOptions} onChange={setSignalId} />
      <NumberField compact label={t('length')} value={length} min={1} onChange={(v) => setLength(v ?? 100)} />
    </div>
    <p className="ui-muted">{t('create_overlap_hint')}</p>
    <div className="ui-row">
      <Button variant="primary" icon={<PlusIcon size={14} />} disabled={!signalId} onClick={() => onCreate(signalId, length)}>{t('create')}</Button>
    </div>
  </>
}

function RouteEditor({ route, name, domain, net, findings, extents, signalOptions, onChange, onDelete, onNewOverlap, onMeasure, onSignal, select, setSketch, showOnPlan, readOnly, t, language }) {
  const { signals, overlaps } = domain.collections
  const set = (patch) => onChange({ ...route, ...patch })
  const start = signals.find(s => s.id === route.startSignalId)
  const target = signals.find(s => s.id === route.targetSignalId)
  const rerun = (patch = {}) => {
    const r = { ...route, ...patch }
    const a = signals.find(s => s.id === r.startSignalId), b = signals.find(s => s.id === r.targetSignalId)
    const run = a && b ? findRun(net, a.at, b.at) : null
    onChange({ ...r, path: { trackIds: run ?? [] } })
  }
  // What it may show: the aspects of its start signal, at its place and its logical signals, with their symbols.
  const shown = [...(start?.frames ?? []), ...(start?.logicalSignals ?? []).flatMap(l => l.frames ?? [])].flatMap(f => f.aspects).filter(a => !['Hp_0', 'Sh_1'].includes(a.aspect) || route.kind === 'shunting')
  const key = (a) => `${a.aspect}|${a.symbol ?? ''}`
  const chosenShows = new Set((route.aspects ?? []).filter(a => a.signalId === route.startSignalId).map(key))
  const atTarget = overlaps.filter(o => o.signalId === route.targetSignalId)
  const named = new Map((route.overlaps ?? []).map(o => [o.overlapId, o]))
  const elements = switchesOnRun(net, route.path.trackIds)
  const speed = signallingSpeedOf(domain, route)
  // Where its speed is set: the logical signal at its start that signals it, else the first.
  const setter = start?.logicalSignals?.find(l => l.signallingSpeeds?.some(v => v.routeId === route.id)) ?? start?.logicalSignals?.[0]
  // What its elements allow, section by section between the Lf 7 on it.
  const sections = route.kind === 'train' && start && target ? routeSpeedSections(net, route, signals) : []
  const derived = sections[0]?.elements ?? null
  // The element shown on the plan is marked over the route's way, till the route is left.
  const way = { ways: [{ pieces: wayPieces(routeStretches(net, route, signals)), strong: true }], ids: [route.startSignalId, route.targetSignalId] }
  useEffect(() => () => setSketch?.(null), [setSketch])

  return (
    <div className="tool-editor">
      <EditorHead title={name} readOnly={readOnly} t={t} onDelete={onDelete} />
      <ObjectFindings findings={findings} collection="routes" id={route.id} t={t} />
      <div className="ui-fields">
        <SelectField compact wide label={t('start_signal')} value={route.startSignalId} options={signalOptions} disabled={readOnly}
          onChange={(v) => rerun({ startSignalId: v, aspects: (route.aspects ?? []).filter(a => a.signalId !== route.startSignalId) })} />
        <SelectField compact wide label={t('target_signal')} value={route.targetSignalId} options={signalOptions} disabled={readOnly}
          onChange={(v) => rerun({ targetSignalId: v, overlaps: [] })} />
        <SelectField compact label={t('route_kind')} value={route.kind} disabled={readOnly}
          options={[{ value: 'train', label: t('train') }, { value: 'shunting', label: t('shunting') }, { value: 'middle', label: t('middle') }]}
          onChange={(v) => set(v === 'train' ? { kind: v, trainType: 'ZZ', shuntingType: undefined, middleType: undefined }
            : v === 'shunting' ? { kind: v, shuntingType: 'RR', trainType: undefined, middleType: undefined, overlaps: undefined }
              : { kind: v, middleType: 'ZM', trainType: undefined, shuntingType: undefined })} />
        {route.kind === 'train' && <SelectField compact label={t('train_type')} value={route.trainType ?? null} empty={t('none')} options={optionsOf(TRAIN_TYPE, language)} disabled={readOnly} onChange={(v) => set({ trainType: v ?? undefined })} />}
        {route.kind === 'shunting' && <SelectField compact label={t('shunting_type')} value={route.shuntingType ?? null} empty={t('none')} options={optionsOf(SHUNTING_TYPE, language)} disabled={readOnly} onChange={(v) => set({ shuntingType: v ?? undefined })} />}
        <NumberField compact label={t('rank')} value={route.rank ?? null} min={0} step={1} disabled={readOnly} onChange={(v) => set({ rank: v ?? undefined })} />
        <TextField compact wide label={t('name')} value={route.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
      </div>
      <Section title={t('route_speed')}>
        <div className="ui-row">
          {setter && !readOnly
            ? <div className="ui-grow"><NumberField compact label={t('route_speed_kmh')} value={speed ?? null} min={0} step={10} onChange={(v) => onSignal(v ?? null)} /></div>
            : <span className="ui-grow">{speed != null ? `${speed} km/h` : t('speed_not_signalled')}</span>}
          {setter && (
            <Button size="small" onClick={() => select({ collection: 'logicalSignals', id: logicalId(start.id, setter.system) })}>
              {t('set_at_signal')}
            </Button>
          )}
        </div>
        {derived && (
          <div className="ui-row">
            <ElementLink net={net} elements={derived} base={way} setSketch={setSketch} showOnPlan={showOnPlan} t={t} />
            {setter && !readOnly && speed !== shownSpeed(derived.speed) && (
              <Button size="small" onClick={() => onSignal(derived.speed)}>{t('take_speed', { speed: shownSpeed(derived.speed) })}</Button>
            )}
          </div>
        )}
        {sections.length > 1 && (
          <ul className="tool-speed-sections">
            {sections.map((x, i) => {
              const shown = i ? speedSignalSpeed(x.signal) : speed
              return (
                <li key={x.signal.id}>
                  <span className="ui-grow">{i ? labelOf('signals', x.signal, t) : labelOf('signals', start, t)} · {Math.round(x.from)}–{Math.round(x.to)} m</span>
                  <span>{shown != null ? `${shown} km/h` : '—'}</span>
                  <span className="ui-muted">{x.elements ? t('speed_sections_elements', { speed: x.elements.speed }) : t('speed_sections_none')}</span>
                </li>
              )
            })}
          </ul>
        )}
      </Section>
      <Section title={t('path')} actions={!readOnly && <Button size="small" onClick={() => rerun()}>{t('find_path')}</Button>}>
        {route.path.trackIds.length
          ? <p className="ui-muted">{route.path.trackIds.map(id => trackName(net, id)).join(' → ')}{elements.length ? ` · ${elements.length} × ${t('switch')}` : ''}
            {start && target && ` · ${t('route_length', { length: Math.round(routeStretches(net, route, signals).reduce((sum, x) => sum + x.hi - x.lo, 0)), from: labelOf('signals', start, t), to: labelOf('signals', target, t) })}`}</p>
          : <p className="ui-message ui-message-warn">{t('path_none')}</p>}
      </Section>
      <Section title={t('route_sections')}>
        <RouteSectionList sections={routeSections(net, route, domain, extents)} t={t} />
      </Section>
      <Section title={`${t('shows')} — ${start ? labelOf('signals', start, t) : ''}`}>
        <div className="tool-checklist">
          {shown.map(a => (
            <Checkbox key={key(a)} label={aspectText(a)} checked={chosenShows.has(key(a))} disabled={readOnly}
              onChange={(on) => {
                const others = (route.aspects ?? []).filter(x => !(x.signalId === route.startSignalId && key(x) === key(a)))
                set({ aspects: on ? [...others, { signalId: route.startSignalId, aspect: a.aspect, ...(a.symbol ? { symbol: a.symbol } : {}) }] : others })
              }} />
          ))}
        </div>
      </Section>
      {route.kind === 'train' && (
        <Section title={`${t('overlaps')} — ${target ? labelOf('signals', target, t) : ''}`}
          actions={!readOnly && target && <>
            <Button size="small" variant="primary" disabled={speed == null} title={t('measure_overlap_hint')} onClick={onMeasure}>{t('measure_overlap')}</Button>
            <Button size="small" icon={<PlusIcon size={12} />} onClick={() => onNewOverlap(target.id)}>{t('new')}</Button>
          </>}>
          {atTarget.length ? atTarget.map(o => (
            <div key={o.id} className="tool-aspect-row">
              <Checkbox label={`${o.name ?? ''} · ${o.length} m${o.gradient != null ? ` · ${o.gradient} ‰` : ''}${o.speed != null ? ` · ${o.speed} km/h` : ''}`} checked={named.has(o.id)} disabled={readOnly}
                onChange={(on) => set({ overlaps: on ? [...(route.overlaps ?? []), { overlapId: o.id }] : (route.overlaps ?? []).filter(x => x.overlapId !== o.id) })} />
              {named.has(o.id) && (
                <Checkbox label={t('preferred_overlap')} checked={named.get(o.id).preferred} disabled={readOnly}
                  onChange={(on) => set({ overlaps: (route.overlaps ?? []).map(x => ({ ...x, preferred: x.overlapId === o.id ? (on || undefined) : (on ? undefined : x.preferred) })) })} />
              )}
            </div>
          )) : <p className="ui-muted">{t('empty_list')}</p>}
        </Section>
      )}
    </div>
  )
}

/** The track sections a route runs through, by name, in the order it meets them. */
function RouteSectionList({ sections, t }) {
  return sections.length
    ? <p className="ui-muted">{sections.map(x => labelOf('trackSections', x, t)).join(' → ')}</p>
    : <p className="ui-muted">{t('route_sections_none')}</p>
}

/**
 * Finding routes from a signal — chosen in the list or on the plan —: every
 * next main signal ahead of it on every way, each a route, with the track
 * sections it runs through and the speed it is run at; those there are
 * already marked. The ways are drawn on the plan. Or the routes of every main
 * signal at once, without speeds.
 */
function FindRoutes({ domain, net, extents, ops, setMode, setSketch, t }) {
  const { signals, routes } = domain.collections
  const mains = signals.filter(s => isMainSignal(s) && s.at?.anchor === 'station')
  const options = signalChoices(net, mains, t)
  const [startId, setStartId] = useState(options[0]?.value ?? null)
  const [speeds, setSpeeds] = useState({})
  const [skipped, setSkipped] = useState(() => new Set())
  const [done, setDone] = useState(null)
  const start = mains.find(s => s.id === startId) ?? null
  const keyOf = (f) => `${f.target.id}|${f.trackIds.join()}`
  const found = useMemo(() => (start ? nextSignals(net, signals, start) : []), [net, signals, start])
  const exists = (startSig, f) => routes.some(r => r.startSignalId === startSig.id && r.targetSignalId === f.target.id && r.path.trackIds.join() === f.trackIds.join())
  const fresh = found.filter(f => !exists(start, f))
  const chosen = fresh.filter(f => !skipped.has(keyOf(f)))

  // The ways found on the plan, each from the signal to the next, and the signals at their ends.
  useEffect(() => {
    setSketch?.(found.length ? {
      ways: found.map(f => ({ pieces: wayPieces(runStretches(net, start.at, f.trackIds, f.target.at)), strong: true })),
      ids: [start.id, ...found.map(f => f.target.id)],
    } : null)
  }, [found, start, net, setSketch])
  useEffect(() => () => setSketch?.(null), [setSketch])

  /** The routes of `ways` from their signals put in the draft, the speeds given signalled, the elements' elsewhere. */
  const make = (ways) => {
    let next = domain
    const made = []
    for (const { start: from, way, speed } of ways) {
      const rank = next.collections.routes.filter(r => r.startSignalId === from.id && r.targetSignalId === way.target.id).length
      const route = {
        id: newId(), kind: 'train', trainType: 'ZZ', startSignalId: from.id, targetSignalId: way.target.id,
        path: { trackIds: way.trackIds }, ...(rank ? { rank } : {}),
      }
      next = added(next, 'routes', route)
      if (speed != null) next = withRouteSpeed(next, route, speed)
      made.push({ route, speed })
    }
    next = withElementSpeeds(next, net, { routeIds: made.filter(m => m.speed == null).map(m => m.route.id), boards: false }).domain
    ops.replaceAll(next)
    return made.length
  }
  const takeChosen = () => {
    const n = make(chosen.map(way => ({ start, way, speed: speeds[keyOf(way)] ?? null })))
    setDone(t('find_routes_done', { n }))
    setSpeeds({})
  }
  const takeAll = () => {
    const ways = mains.flatMap(s => nextSignals(net, signals, s).filter(f => !exists(s, f)).map(way => ({ start: s, way, speed: null })))
    setDone(t('find_routes_done_all', { n: make(ways), signals: mains.length }))
  }
  const km = (m) => (m >= 1000 ? `${(m / 1000).toLocaleString('de', { maximumFractionDigits: 2 })} km` : `${Math.round(m)} m`)

  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('start_signal')} value={startId} options={options} onChange={(v) => { setStartId(v); setSkipped(new Set()); setDone(null) }} />
      <div className="ui-wide ui-row">
        <Button size="small" disabled={!mains.length} onClick={() => setMode(pickingSignal(mains, t, (id) => { setStartId(id); setSkipped(new Set()); setDone(null); setMode(null) }))}>
          {t('pick_signal_on_plan')}
        </Button>
      </div>
    </div>
    {done && <p className="tool-created" role="status">{done}</p>}
    {start && !found.length && <p className="ui-muted">{t('find_routes_none')}</p>}
    {found.map(f => {
      const key = keyOf(f)
      const there = exists(start, f)
      const route = { startSignalId: start.id, targetSignalId: f.target.id, path: { trackIds: f.trackIds } }
      const derived = shownSpeed(routeElementSpeed(net, route, signals)?.speed)
      return (
        <div key={key} className="tool-found-route">
          <div className="ui-row">
            <span className="ui-grow">
              <Checkbox label={`→ ${signalDesignation(f.target, t)} · ${km(f.length)}${there ? ` · ${t('find_routes_there')}` : ''}`}
                checked={!there && !skipped.has(key)} disabled={there}
                onChange={(on) => setSkipped(prev => { const n = new Set(prev); if (on) n.delete(key); else n.add(key); return n })} />
            </span>
            {!there && (
              <NumberField compact label={derived != null && speeds[key] == null ? t('route_speed_from_elements') : t('route_speed_kmh')} value={speeds[key] ?? derived} min={0} step={10}
                onChange={(v) => setSpeeds(prev => ({ ...prev, [key]: v ?? undefined }))} />
            )}
          </div>
          <RouteSectionList sections={routeSections(net, route, domain, extents)} t={t} />
        </div>
      )
    })}
    <div className="ui-row">
      <Button variant="primary" disabled={!chosen.length} onClick={takeChosen}>{t('find_routes_make', { n: chosen.length })}</Button>
      <Button disabled={!mains.length} onClick={takeAll}>{t('find_routes_all')}</Button>
    </div>
    <p className="ui-muted">{t('find_routes_hint')}</p>
  </>
}

function OverlapEditor({ overlap, domain, net, findings, extents, signalOptions, onChange, onDelete, setNotice, readOnly, t }) {
  const { signals, switchElements, trackSections } = domain.collections
  const set = (patch) => onChange({ ...overlap, ...patch })
  const rerun = (patch = {}) => {
    const o = { ...overlap, ...patch }
    const s = signals.find(x => x.id === o.signalId)
    const run = s ? runAhead(net, s.at, o.length) : null
    onChange(run ? { ...o, path: { trackIds: run.trackIds }, end: { anchor: 'station', ...run.end } } : o)
  }
  const onRun = new Set(switchesOnRun(net, overlap.path.trackIds))
  const elementsOnRun = switchElements.filter(e => onRun.has(e.switchId))
  const locked = new Map((overlap.switches ?? []).map(s => [s.switchElementId, s.locked]))
  const near = trackSections.filter(s => (extents.get(s.id)?.parts ?? []).some(p => overlap.path.trackIds.includes(p.trackId)))
  const forSpeed = overlapSpeed(domain, overlap)
  const measure = () => {
    const m = measuredOverlap(net, signals.find(x => x.id === overlap.signalId), forSpeed)
    if (!m) setNotice?.({ tone: 'warn', text: t(forSpeed == null ? 'measure_no_speed' : 'measure_no_target') })
    else {
      const { gradient: _before, ...rest } = overlap
      onChange({ ...rest, ...m })
    }
  }
  return (
    <div className="tool-editor">
      <EditorHead title={`${labelOf('signals', signals.find(s => s.id === overlap.signalId), t)} ${overlap.name ?? ''}`} readOnly={readOnly} t={t} onDelete={onDelete} />
      <ObjectFindings findings={findings} collection="overlaps" id={overlap.id} t={t} />
      <div className="ui-fields">
        <SelectField compact wide label={t('at_signal')} value={overlap.signalId} options={signalOptions} disabled={readOnly} onChange={(v) => rerun({ signalId: v })} />
        <TextField compact label={t('name')} value={overlap.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <NumberField compact label={t('length')} value={overlap.length} min={0} disabled={readOnly} onChange={(v) => v != null && rerun({ length: v })} />
        <NumberField compact label={t('gradient')} value={overlap.gradient ?? null} disabled={readOnly} onChange={(v) => set({ gradient: v ?? undefined })} />
        <NumberField compact label={t('speed')} value={overlap.speed ?? null} min={0} disabled={readOnly} onChange={(v) => set({ speed: v ?? undefined })} />
        <NumberField compact label={t('release_delay')} value={overlap.releaseDelay ?? null} min={0} disabled={readOnly} onChange={(v) => set({ releaseDelay: v ?? undefined })} />
      </div>
      {!readOnly && (
        <div className="ui-row">
          <Button size="small" variant="primary" disabled={forSpeed == null} onClick={measure}>{t('measure_overlap')}</Button>
          <span className="ui-muted">{forSpeed != null ? t('measure_for', { speed: forSpeed }) : t('speed_not_signalled')}</span>
        </div>
      )}
      <Section title={t('path')} actions={!readOnly && <Button size="small" onClick={() => rerun()}>{t('rerun')}</Button>}>
        <p className="ui-muted">{overlap.path.trackIds.map(id => trackName(net, id)).join(' → ')}</p>
        <p className="ui-muted">{t('danger_point')}: {trackName(net, overlap.end?.trackId)} · {Math.round(overlap.end?.station ?? 0)} m</p>
      </Section>
      {elementsOnRun.length > 0 && (
        <Section title={t('section_elements')}>
          {elementsOnRun.map(e => (
            <Checkbox key={e.id} label={`${labelOf('switchElements', e, t)} — ${t('locked')}`} checked={locked.get(e.id)} disabled={readOnly}
              onChange={(on) => set({ switches: [...(overlap.switches ?? []).filter(s => s.switchElementId !== e.id), { switchElementId: e.id, locked: on }] })} />
          ))}
        </Section>
      )}
      <Section title={t('clear_sections')}>
        <CheckList readOnly={readOnly} value={overlap.clearSections} onChange={(v) => set({ clearSections: v.length ? v : undefined })}
          options={near.map(s => ({ value: s.id, label: labelOf('trackSections', s, t) }))} />
      </Section>
    </div>
  )
}
