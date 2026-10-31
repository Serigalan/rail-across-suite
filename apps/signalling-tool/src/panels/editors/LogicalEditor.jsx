import { NumberField, Section, SelectField } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, signalKindOf } from '@rail-across/core/signalling'
import { labelOf, logicalId, withKind, withLogical, withSignallingSpeed } from '../../model/domain.js'
import { FUNCTION, logicalSignalWord, optionsOf, signalSystemOptions, signalTypeOptions } from '../../labels.js'
import { CheckList, EditorHead, ObjectFindings } from '../common.jsx'
import FramesEditor from '../frames.jsx'

/**
 * A logical signal — what a signal mast is in one signalling system, as OSRD
 * has it: the mast it belongs to, its system and kind, its function, what its
 * screen and indicators show, the systems it announces, and the speed it
 * signals for each route that starts at its mast (signallingSpeed; the design
 * speed of the track is the Layout-Tool's).
 */
export default function LogicalEditor({ id, found, domain, findings, select, ops, readOnly, t, language }) {
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
