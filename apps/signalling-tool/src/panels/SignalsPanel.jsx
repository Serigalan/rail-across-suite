import { Badge, Button, NumberField, Panel, PlusIcon, Section, SelectField, SelectList } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, signalKindOf } from '@rail-across/core/signalling'
import {
  allLogical, findLogical, labelOf, logicalId, newSignal, withKind, withLogical, withSignallingSpeed,
} from '../model/domain.js'
import { FUNCTION, logicalSignalWord, optionsOf, signalSystemOptions, signalTypeOptions, word } from '../labels.js'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'
import FramesEditor from './frames.jsx'
import { OFFSET, locationFrom, placing, trackName } from '../model/places.js'

/**
 * The logical signals — what each place is in each signalling system it
 * shows, as OSRD has them: the place it belongs to, its system and kind, its
 * function, what its screen and indicators show, the systems it announces,
 * and the speed it signals for each route that starts at its place
 * (signallingSpeed; the design speed of the track is the Layout-Tool's). A
 * new one is set by a click on the plan with a place of its own, or added to
 * a place chosen in Standorte.
 */
export default function SignalsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const all = allLogical(domain)
  const chosen = selection?.collection === 'logicalSignals' ? findLogical(domain, selection.id) : {}
  const badPlaces = new Set(findings.errors.filter(f => f.collection === 'signals').map(f => f.key))
  const hasError = (e) => [...badPlaces].some(k => k.includes(`:${e.signal.id}:logicalSignals.${e.index}`))
  const items = all.map(e => ({
    id: e.id,
    title: `${labelOf('signals', e.signal, t)} · ${logicalSignalWord(e.logical, language)}`,
    sub: [word(FUNCTION, e.logical.function, language),
      e.signal.at?.anchor === 'station' ? `${trackName(net, e.signal.at.trackId)} · ${Math.round(e.signal.at.station)} m` : t('orphan')].filter(Boolean).join(' · '),
    end: hasError(e) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))

  const placeNew = () => setMode(placing('signals', (hit) => newSignal({ at: locationFrom(hit, { offset: OFFSET.signals }) }), (s) => {
    ops.add('signals', s)
    select({ collection: 'logicalSignals', id: logicalId(s.id, s.logicalSignals[0].system) })
  }))

  return (
    <Panel title={t('section_signals')} intro={t('section_signals_intro')}
      actions={!readOnly && (
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length} onClick={placeNew}>{t('new_signal')}</Button>
      )}>
      <SelectList maxHeight="34vh" label={t('section_signals')} items={items} selected={chosen.signal ? selection.id : null}
        onSelect={(id) => select({ collection: 'logicalSignals', id })} empty={t('empty_list')} />
      {chosen.signal && (
        <LogicalEditor key={selection.id} id={selection.id} found={chosen} domain={domain} findings={findings} readOnly={readOnly} t={t} language={language}
          select={select} ops={ops} />
      )}
    </Panel>
  )
}

function LogicalEditor({ id, found, domain, findings, select, ops, readOnly, t, language }) {
  const { signal, logical, index } = found
  const { signals, routes } = domain.collections
  const change = (next) => {
    ops.replaceAll(withLogical(domain, id, next))
    if (next.system !== logical.system) select({ collection: 'logicalSignals', id: logicalId(signal.id, next.system) })
  }
  const set = (patch) => change({ ...logical, ...patch })
  const systems = signalSystemOptions(language)
  const taken = new Set(signal.logicalSignals.map(l => l.system))
  const firstType = (system) => Object.keys(SIGNAL_CATALOGUE.systeme[system]?.signalarten ?? {})[0]
  const kind = signalKindOf(logical.system, logical.type)
  // It moves to another place where that has no logical signal of its system yet.
  const places = signals.filter(s => s.kind === 'real' && (s.id === signal.id || !(s.logicalSignals ?? []).some(l => l.system === logical.system)))
  const moveTo = (placeId) => {
    if (placeId === signal.id) return
    // The speeds it signals were for routes from its old place: they stay behind.
    const { signallingSpeeds: _left, ...moving } = logical
    const without = withLogical(domain, id, null)
    ops.replaceAll({
      ...without,
      collections: {
        ...without.collections,
        signals: without.collections.signals.map(s => (s.id === placeId ? { ...s, logicalSignals: [...(s.logicalSignals ?? []), moving] } : s)),
      },
    })
    select({ collection: 'logicalSignals', id: logicalId(placeId, logical.system) })
  }
  const fromHere = routes.filter(r => r.startSignalId === signal.id)
  const speedOf = (routeId) => logical.signallingSpeeds?.find(v => v.routeId === routeId)?.speed ?? null
  const routeName = (r) => r.name ?? `${labelOf('signals', signal, t)} → ${labelOf('signals', signals.find(s => s.id === r.targetSignalId), t)}`

  return (
    <div className="tool-editor">
      <EditorHead title={`${labelOf('signals', signal, t)} · ${logicalSignalWord(logical, language)}`} readOnly={readOnly} t={t}
        onDelete={() => ops.remove('logicalSignals', id)} />
      <ObjectFindings findings={findings} collection="signals" id={signal.id} t={t} only={(f) => f.key.includes(`:logicalSignals.${index}`)} />
      <div className="ui-fields">
        <SelectField compact wide label={t('location_of_signal')} value={signal.id} disabled={readOnly}
          options={places.map(s => ({ value: s.id, label: labelOf('signals', s, t) }))} onChange={moveTo} />
        <SelectField compact wide label={t('system')} value={logical.system} disabled={readOnly}
          options={systems.filter(o => o.value === logical.system || !taken.has(o.value))}
          onChange={(v) => change(withKind(logical, v, firstType(v)))} />
        <SelectField compact wide label={t('type')} value={logical.type ?? null} empty={t('none')} disabled={readOnly}
          options={signalTypeOptions(logical.system, language, t)} onChange={(v) => change(withKind(logical, logical.system, v ?? undefined))} />
        <SelectField compact wide label={t('function')} value={logical.function ?? null} empty={t('none')} disabled={readOnly}
          options={optionsOf(FUNCTION, language)} onChange={(v) => set({ function: v ?? undefined })} />
      </div>

      <Section title={t('signalling_speeds')}>
        <p className="ui-muted">{t('signalling_speeds_hint')}</p>
        {fromHere.length ? fromHere.map(r => (
          <div key={r.id} className="ui-row tool-speed-row">
            <button type="button" className="ui-link ui-grow" onClick={() => select({ collection: 'routes', id: r.id })}>{routeName(r)}</button>
            <NumberField compact label={t('signalling_speed')} value={speedOf(r.id)} min={0} step={5} disabled={readOnly}
              onChange={(v) => change(withSignallingSpeed(logical, r.id, v))} />
          </div>
        )) : <p className="ui-muted">{t('no_routes_from_here')}</p>}
      </Section>

      <FramesEditor frames={logical.frames} kind={kind} adds={['indicator']} readOnly={readOnly} t={t} language={language}
        onChange={(frames) => set({ frames: frames.length ? frames : undefined })} />

      <Section title={t('next_systems')}>
        <CheckList options={systems} value={logical.nextSystems} readOnly={readOnly}
          onChange={(next) => set({ nextSystems: next.length ? next : undefined })} />
      </Section>
    </div>
  )
}
