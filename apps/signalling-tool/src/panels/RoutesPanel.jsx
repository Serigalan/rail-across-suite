import { useState } from 'react'
import {
  Badge, Button, Checkbox, GroupedList, NumberField, Option, Options, OptionsDivider, OptionsHeading, Panel, PlusIcon, Section, SelectField, TextField,
} from '@rail-across/ui'
import { signallingSpeedOf } from '@rail-across/core/signalling'
import { newId, labelOf, logicalId } from '../model/domain.js'
import { measuredOverlap, overlapSpeed, withMeasuredOverlap } from '../model/overlaps.js'
import { findRun, runAhead, switchesOnRun } from '../model/runs.js'
import { SHUNTING_TYPE, TRAIN_TYPE, optionsOf } from '../labels.js'
import { OverlapIcon, RouteIcon } from '../icons.jsx'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'
import { aspectText, trackName } from '../model/places.js'

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
 */
export default function RoutesPanel({ domain, net, findings, extents, selection, select, ops, readOnly, t, language, setNotice }) {
  const [page, setPage] = useState('menu')
  const { routes, overlaps, signals } = domain.collections
  const sig = (id) => signals.find(s => s.id === id)
  const sigName = (id) => labelOf('signals', sig(id), t)
  const routeName = (r) => r.name ?? `${sigName(r.startSignalId)} → ${sigName(r.targetSignalId)}`
  const bad = new Set(findings.errors.filter(f => f.collection === 'routes' || f.collection === 'overlaps').map(f => f.id))
  const chosenRoute = selection?.collection === 'routes' ? routes.find(r => r.id === selection.id) : null
  const chosenOverlap = selection?.collection === 'overlaps' ? overlaps.find(o => o.id === selection.id) : null
  const back = { label: t('back'), onClick: () => setPage('menu') }
  const toPage = { label: t('back'), onClick: () => select(null) }

  const real = signals.filter(s => s.at?.anchor === 'station')
  const signalOptions = real.map(s => ({ value: s.id, label: labelOf('signals', s, t) })).sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))

  const newOverlap = (signalId, length = 100) => {
    const s = sig(signalId)
    if (!s) return
    const run = runAhead(net, s.at, length)
    const name = `D${overlaps.filter(o => o.signalId === signalId).length + 1}`
    setPage('overlaps')
    ops.add('overlaps', { id: newId(), name, signalId, path: { trackIds: run.trackIds }, end: { anchor: 'station', ...run.end }, length })
  }

  if (chosenRoute) {
    return (
      <Panel title={t('object_routes')} back={toPage}>
        <RouteEditor key={chosenRoute.id} route={chosenRoute} name={routeName(chosenRoute)} domain={domain} net={net} findings={findings}
          signalOptions={signalOptions} readOnly={readOnly} t={t} language={language} select={select}
          onChange={(r) => ops.update('routes', r)} onDelete={() => ops.remove('routes', chosenRoute.id)} onNewOverlap={newOverlap}
          onMeasure={() => {
            const { domain: next, problem } = withMeasuredOverlap(domain, net, chosenRoute)
            if (next) ops.replaceAll(next)
            else setNotice?.({ tone: 'warn', text: t(`measure_no_${problem}`) })
          }} />
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
            setPage('routes')
            ops.add('routes', { ...r, id: newId(), path: { trackIds: run ?? [] } })
          }} />
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
      sub: [r.trainType ?? r.shuntingType ?? r.middleType, signallingSpeedOf(domain, r) != null ? `${signallingSpeedOf(domain, r)} km/h` : null, r.rank ? `#${r.rank}` : null].filter(Boolean).join(' · '),
      end: bad.has(r.id) ? <Badge tone="danger">!</Badge> : null,
    })
    return (
      <Panel title={t('section_routes')} intro={t('routes_list_intro')} back={back}>
        <GroupedList label={t('section_routes')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('empty_list')}
          groups={Object.keys(KINDS).map(kind => ({
            key: kind, title: t(`routes_${kind}`),
            items: routes.filter(r => r.kind === kind).map(item).sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true })),
          }))}
          onSelect={(id) => select({ collection: 'routes', id })} />
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
          <Option icon={<RouteIcon />} disabled={real.length < 2} onClick={() => setPage('new_route')}>{t('create_route')}</Option>
          <Option icon={<OverlapIcon />} disabled={!real.length} onClick={() => setPage('new_overlap')}>{t('create_overlap')}</Option>
        </Options>
        <OptionsDivider />
      </>}
      <Options>
        <OptionsHeading>{t('section_edit')}</OptionsHeading>
        <Option icon={<RouteIcon />} end={routes.length} onClick={() => setPage('routes')}>{t('section_routes')}</Option>
        <Option icon={<OverlapIcon />} end={overlaps.length} onClick={() => setPage('overlaps')}>{t('overlaps')}</Option>
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

function RouteEditor({ route, name, domain, net, findings, signalOptions, onChange, onDelete, onNewOverlap, onMeasure, select, readOnly, t, language }) {
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
          <span className="ui-grow">{speed != null ? `${speed} km/h` : t('speed_not_signalled')}</span>
          {setter && (
            <Button size="small" onClick={() => select({ collection: 'logicalSignals', id: logicalId(start.id, setter.system) })}>
              {t('set_at_signal')}
            </Button>
          )}
        </div>
      </Section>
      <Section title={t('path')} actions={!readOnly && <Button size="small" onClick={() => rerun()}>{t('find_path')}</Button>}>
        {route.path.trackIds.length
          ? <p className="ui-muted">{route.path.trackIds.map(id => trackName(net, id)).join(' → ')}{elements.length ? ` · ${elements.length} × ${t('switch')}` : ''}</p>
          : <p className="ui-message ui-message-warn">{t('path_none')}</p>}
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
