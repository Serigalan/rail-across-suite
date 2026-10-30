import { useState } from 'react'
import { Badge, Button, Checkbox, NumberField, Panel, PlusIcon, Section, SelectField, SelectList, Tabs, TextField } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { findRun, runAhead, switchesOnRun } from '../model/runs.js'
import { SHUNTING_TYPE, TRAIN_TYPE, optionsOf } from '../labels.js'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'
import { aspectText, trackName } from '../model/places.js'

/**
 * Routes and their overlaps. A new route runs from its start signal to its
 * target over the shortest run the net has the way the two act; it shows the
 * aspects of its start signal that are chosen, and names the overlaps at its
 * target. A new overlap runs its length ahead of its signal, straight on at
 * every switch; its danger point lies at its end.
 */
export default function RoutesPanel({ domain, net, findings, extents, selection, select, ops, readOnly, t, language }) {
  const [tab, setTab] = useState(selection?.collection === 'overlaps' ? 'overlaps' : 'routes')
  const { routes, overlaps, signals } = domain.collections
  const sig = (id) => signals.find(s => s.id === id)
  const sigName = (id) => labelOf('signals', sig(id), t)
  const routeName = (r) => r.name ?? `${sigName(r.startSignalId)} → ${sigName(r.targetSignalId)}`
  const bad = new Set(findings.errors.filter(f => f.collection === 'routes' || f.collection === 'overlaps').map(f => f.id))
  const chosenRoute = selection?.collection === 'routes' ? routes.find(r => r.id === selection.id) : null
  const chosenOverlap = selection?.collection === 'overlaps' ? overlaps.find(o => o.id === selection.id) : null

  const routeItems = routes.map(r => ({
    id: r.id, title: routeName(r),
    sub: [r.kind === 'train' ? `${t('train')} ${r.trainType ?? ''}` : r.kind === 'middle' ? t('middle') : `${t('shunting')} ${r.shuntingType ?? ''}`,
      r.speed ? `${r.speed} km/h` : null, r.rank ? `#${r.rank}` : null].filter(Boolean).join(' · '),
    end: bad.has(r.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => a.title.localeCompare(b.title, 'de', { numeric: true }))
  const overlapItems = overlaps.map(o => ({
    id: o.id, title: `${sigName(o.signalId)} ${o.name ?? ''}`.trim(), sub: `${o.length} m${o.speed ? ` · ${o.speed} km/h` : ''}`,
    end: bad.has(o.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => a.title.localeCompare(b.title, 'de', { numeric: true }))

  const real = signals.filter(s => s.at?.anchor === 'station')
  const signalOptions = real.map(s => ({ value: s.id, label: labelOf('signals', s, t) })).sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))

  const newRoute = () => {
    const [a, b] = real
    const route = { id: newId(), kind: 'train', trainType: 'ZZ', startSignalId: a?.id, targetSignalId: b?.id ?? a?.id, path: { trackIds: [] } }
    const run = a && b ? findRun(net, a.at, b.at) : null
    ops.add('routes', run ? { ...route, path: { trackIds: run } } : route)
    setTab('routes')
  }
  const newOverlap = (signalId) => {
    const s = sig(signalId)
    if (!s) return
    const run = runAhead(net, s.at, 100)
    ops.add('overlaps', { id: newId(), name: 'D1', signalId, path: { trackIds: run.trackIds }, end: { anchor: 'station', ...run.end }, length: 100 })
    setTab('overlaps')
  }

  return (
    <Panel title={t('section_routes')} intro={t('section_routes_intro')}
      actions={!readOnly && <>
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={real.length < 2} onClick={newRoute}>{t('new_route')}</Button>
        <Button size="small" icon={<PlusIcon size={14} />} disabled={!real.length}
          onClick={() => newOverlap(chosenRoute?.targetSignalId ?? (selection?.collection === 'signals' ? selection.id : real[0]?.id))}>{t('new_overlap')}</Button>
      </>}>
      <Tabs label={t('section_routes')} value={tab} onChange={setTab}
        options={[{ value: 'routes', label: `${t('section_routes')} (${routes.length})` }, { value: 'overlaps', label: `${t('overlaps')} (${overlaps.length})` }]} />
      {tab === 'routes'
        ? <SelectList maxHeight="34vh" label={t('section_routes')} items={routeItems} selected={chosenRoute?.id} onSelect={(id) => select({ collection: 'routes', id })} empty={t('empty_list')} />
        : <SelectList maxHeight="34vh" label={t('overlaps')} items={overlapItems} selected={chosenOverlap?.id} onSelect={(id) => select({ collection: 'overlaps', id })} empty={t('empty_list')} />}
      {chosenRoute && (
        <RouteEditor key={chosenRoute.id} route={chosenRoute} name={routeName(chosenRoute)} domain={domain} net={net} findings={findings}
          signalOptions={signalOptions} readOnly={readOnly} t={t} language={language}
          onChange={(r) => ops.update('routes', r)} onDelete={() => ops.remove('routes', chosenRoute.id)} onNewOverlap={newOverlap} />
      )}
      {chosenOverlap && (
        <OverlapEditor key={chosenOverlap.id} overlap={chosenOverlap} domain={domain} net={net} findings={findings} extents={extents}
          signalOptions={signalOptions} readOnly={readOnly} t={t}
          onChange={(o) => ops.update('overlaps', o)} onDelete={() => ops.remove('overlaps', chosenOverlap.id)} />
      )}
    </Panel>
  )
}

function RouteEditor({ route, name, domain, net, findings, signalOptions, onChange, onDelete, onNewOverlap, readOnly, t, language }) {
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
  // What it may show: the aspects of its start signal, with their symbols.
  const shown = (start?.frames ?? []).flatMap(f => f.aspects).filter(a => !['Hp_0', 'Sh_1'].includes(a.aspect) || route.kind === 'shunting')
  const key = (a) => `${a.aspect}|${a.symbol ?? ''}`
  const chosenShows = new Set((route.aspects ?? []).filter(a => a.signalId === route.startSignalId).map(key))
  const atTarget = overlaps.filter(o => o.signalId === route.targetSignalId)
  const named = new Map((route.overlaps ?? []).map(o => [o.overlapId, o]))
  const elements = switchesOnRun(net, route.path.trackIds)

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
        <NumberField compact label={t('speed')} value={route.speed ?? null} min={0} disabled={readOnly} onChange={(v) => set({ speed: v ?? undefined })} />
        <NumberField compact label={t('rank')} value={route.rank ?? null} min={0} step={1} disabled={readOnly} onChange={(v) => set({ rank: v ?? undefined })} />
        <TextField compact wide label={t('name')} value={route.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
      </div>
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
          actions={!readOnly && target && <Button size="small" icon={<PlusIcon size={12} />} onClick={() => onNewOverlap(target.id)}>{t('new')}</Button>}>
          {atTarget.length ? atTarget.map(o => (
            <div key={o.id} className="tool-aspect-row">
              <Checkbox label={`${o.name ?? ''} · ${o.length} m`} checked={named.has(o.id)} disabled={readOnly}
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

function OverlapEditor({ overlap, domain, net, findings, extents, signalOptions, onChange, onDelete, readOnly, t }) {
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
