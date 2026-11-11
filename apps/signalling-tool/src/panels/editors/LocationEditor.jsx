import { Button, Checkbox, NumberField, Option, Options, Section, SelectField, TextField } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, signallingSpeedOf } from '@rail-across/core/signalling'
import { labelOf, logicalId, newLogical, withRouteSpeed } from '../../model/domain.js'
import { VIRTUAL_FUNCTION, functionOptions, functionWord, logicalSignalWord, mountingOptions, optionsOf } from '../../labels.js'
import { SignalIcon } from '../../icons.jsx'
import { EditorHead, LocationFields, ObjectFindings, PlaneLine, SpeedFigureField } from '../common.jsx'
import FramesEditor from '../frames.jsx'
import { SpeedReach } from '../speedReach.jsx'
import { BRACKET_OFFSET, FOUNDATION_OFFSET, ON_FOUNDATION, bracketScreen, farMastPoint, foundationPoint, gantryFarOffset, sideAlong, trackName } from '../../model/places.js'
import { SPEED_SIGNAL, isSpeedSignal, speedSignalElementSpeed, speedSignalSpeed, speedSymbol, withSpeedSignalSpeed } from '../../model/routeSpeeds.js'
import { stationName } from '../../model/stations.js'

/**
 * A signal mast — the physical signal: where it stands, on what mounting,
 * from how far it is seen, the station it belongs to or the open line, the
 * boards it carries of no system, and its logical signals, each opened in an
 * editor of its own; a virtual signal has its function instead. On a gantry
 * or a bracket its location is its screen's, and its foundation has its own.
 * A speed signal Lf 7 says how far it holds and what the elements there
 * allow; a signal with routes from it, the Zs 3 that shows their speeds.
 */
export default function LocationEditor({ signal, domain, net, core, findings, ops, onChange, onDelete, onPlace, select, setSketch, showOnPlan, readOnly, t, language }) {
  const set = (patch) => onChange({ ...signal, ...patch })
  const logical = signal.logicalSignals ?? []
  const free = Object.keys(SIGNAL_CATALOGUE.systeme).filter(sys => !logical.some(l => l.system === sys))
  const addLogical = (system) => {
    const type = Object.keys(SIGNAL_CATALOGUE.systeme[system].signalarten)[0]
    onChange({ ...signal, logicalSignals: [...logical, newLogical({ system, type, fn: logical[0]?.function ?? 'entry' })] })
    select({ collection: 'logicalSignals', id: logicalId(signal.id, system) })
  }
  return (
    <div className="tool-editor">
      <EditorHead title={labelOf('signals', signal, t)} onDelete={onDelete} readOnly={readOnly} t={t} />
      <ObjectFindings findings={findings} collection="signals" id={signal.id} t={t} only={(f) => !f.key.includes(':logicalSignals.')} />
      <div className="ui-fields">
        <TextField compact label={t('name')} value={signal.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <TextField compact label={t('area')} value={signal.areaNumber ?? ''} disabled={readOnly} onChange={(v) => set({ areaNumber: v || undefined })} />
        <SelectField compact wide label={t('kind')} value={signal.kind} disabled={readOnly}
          options={[{ value: 'real', label: t('real') }, { value: 'virtual', label: t('virtual') }]}
          onChange={(v) => set(v === 'virtual'
            ? { kind: 'virtual', frames: undefined, logicalSignals: undefined, function: undefined, mounting: undefined, virtualFunction: signal.virtualFunction ?? 'trainTargetLine' }
            : { kind: 'real', virtualFunction: undefined, frames: [], mounting: 'mast' })} />
        {signal.kind === 'virtual' ? (
          <SelectField compact wide label={t('virtual_function')} value={signal.virtualFunction ?? null} disabled={readOnly}
            options={optionsOf(VIRTUAL_FUNCTION, language)} onChange={(v) => set({ virtualFunction: v ?? undefined })} />
        ) : <>
          {!logical.length && (
            <SelectField compact wide label={t('function')} value={signal.function ?? null} empty={t('none')} disabled={readOnly}
              options={functionOptions(language, signal.function, { repeater: false })} onChange={(v) => set({ function: v ?? undefined })} />
          )}
          <SelectField compact label={t('mounting')} value={signal.mounting ?? null} empty={t('none')} disabled={readOnly}
            options={mountingOptions(language, signal.mounting)} onChange={(v) => {
              // A gantry or a bracket stands on a foundation, on the side the signal stood; nothing else has one.
              const foundation = ON_FOUNDATION.has(v) ? signal.foundation ?? { side: sideAlong(signal.at, signal.at?.side) ?? 'right', offset: FOUNDATION_OFFSET } : undefined
              set({ mounting: v ?? undefined, foundation, ...(v === 'bracket' ? { at: bracketAt(signal.at, foundation) } : {}) })
            }} />
          <SelectField compact wide label={t('station_of_signal')} value={signal.operationalPointId ?? null} empty={t('open_line')} disabled={readOnly}
            options={(core?.collections.operationalPoints ?? []).map(p => ({ value: p.id, label: stationName(p) }))
              .sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true }))}
            onChange={(v) => set({ operationalPointId: v ?? undefined })} />
          <div className="ui-check-cell"><Checkbox label={t('both_directions')} checked={signal.bothDirections} disabled={readOnly} onChange={(v) => set({ bothDirections: v || undefined })} /></div>
        </>}
      </div>

      {isSpeedSignal(signal) && <SpeedSignalSection signal={signal} domain={domain} net={net} set={onChange} setSketch={setSketch} showOnPlan={showOnPlan} readOnly={readOnly} t={t} />}
      {logical.length > 0 && <Zs3Section signal={signal} domain={domain} ops={ops} select={select} readOnly={readOnly} t={t} />}

      {signal.kind === 'real' && (
        <Section title={t('logical_signals')}>
          <p className="ui-muted">{t('logical_signals_hint')}</p>
          <Options>
            {logical.map(l => (
              <Option key={l.system} icon={<SignalIcon />} end={l.function ? functionWord(l.function, l.type, language) : null}
                onClick={() => select({ collection: 'logicalSignals', id: logicalId(signal.id, l.system) })}>
                {logicalSignalWord(l, language)}
              </Option>
            ))}
          </Options>
          {!readOnly && free.length > 0 && (
            <select className="ui-input" aria-label={t('add_logical_signal')} value=""
              onChange={(e) => { if (e.target.value) addLogical(e.target.value) }}>
              <option value="">+ {t('add_logical_signal')}</option>
              {free.map(sys => <option key={sys} value={sys}>{logicalSignalWord({ system: sys }, language)}</option>)}
            </select>
          )}
        </Section>
      )}

      <Section title={signal.foundation ? t('screen_location') : t('location')}>
        <LocationFields loc={signal.at} net={net} t={t} readOnly={readOnly} onPlace={onPlace} onChange={(at) => set({ at })} />
        {(signal.alsoAt ?? []).map((also, i) => (
          <p key={i} className="ui-muted">+ {trackName(net, also.trackId)} · {Math.round(also.station)} m</p>
        ))}
      </Section>

      {signal.foundation && (
        <Section title={t('foundation_location')}>
          <div className="ui-fields">
            <SelectField compact label={t('side_along')} value={signal.foundation.side ?? 'right'} disabled={readOnly}
              options={[{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]}
              onChange={(v) => {
                const foundation = { ...signal.foundation, side: v ?? 'right' }
                // A bracket's signal goes with it to the other side.
                set({ foundation, ...(signal.mounting === 'bracket' ? { at: bracketAt(signal.at, foundation) } : {}) })
              }} />
            <NumberField compact label={t('offset')} value={signal.foundation.offset} min={0} disabled={readOnly}
              onChange={(v) => set({ foundation: { ...signal.foundation, offset: v ?? FOUNDATION_OFFSET } })} />
            <PlaneLine point={foundationPoint(net, signal)} t={t} />
          </div>
          {signal.mounting === 'gantry' && <>
            <p className="ui-muted">{t('far_mast', farText(signal, t))}</p>
            <PlaneLine point={farMastPoint(net, signal)} t={t} />
          </>}
        </Section>
      )}

      {signal.kind === 'real' && <>
        <Section title={t('sight')}>
          <div className="ui-fields tool-fields-3">
            {['required', 'minimum', 'achievable'].map(k => (
              <NumberField key={k} compact label={t(`sight_${k}`)} value={signal.sight?.[k] ?? null} min={0} disabled={readOnly}
                onChange={(v) => {
                  const sight = { ...(signal.sight ?? {}), [k]: v ?? undefined }
                  set({ sight: Object.values(sight).some(x => x != null) ? sight : undefined })
                }} />
            ))}
          </div>
        </Section>
        <FramesEditor title={t('frames_of_place')} frames={signal.frames} kind={null} readOnly={readOnly} t={t} language={language}
          adds={logical.length ? ['plate', 'body', 'nameplate'] : ['screen', 'plate', 'body', 'nameplate']}
          onChange={(frames) => set({ frames })} />
      </>}
    </div>
  )
}

/** A bracket's signal: on the other side of the track than its foundation, at the offset it has there, else the bracket's. */
function bracketAt(at, foundation) {
  const screen = bracketScreen(foundation, at?.offset || BRACKET_OFFSET)
  return { ...at, side: sideAlong(at, screen.side), offset: screen.offset }
}

/** Where a gantry's second mast stands, for people: its offset and side seen the way the signal acts. */
function farText(signal, t) {
  const far = gantryFarOffset(signal)
  const side = sideAlong(signal.at, far < 0 ? 'left' : 'right')
  return { offset: Math.round(Math.abs(far) * 100) / 100, side: t(side === 'left' ? 'left_of' : 'right_of') }
}

/** What an Lf 7 shows — its figure, chosen here as in its frame —, and what the elements from it to the next main signal or Lf 7 allow. */
function SpeedSignalSection({ signal, domain, net, set, setSketch, showOnPlan, readOnly, t }) {
  const shown = speedSignalSpeed(signal)
  const derived = speedSignalElementSpeed(net, domain.collections.signals, signal)
  const fits = derived && speedSymbol(derived.speed) === String((shown ?? 0) / 10)
  return (
    <Section title={t('speed_signal_section')}>
      <div className="ui-fields">
        <SpeedFigureField aspect={SPEED_SIGNAL} label={t('lf7_shown')} speed={shown} readOnly={readOnly} t={t}
          onChange={(v) => set(withSpeedSignalSpeed(signal, v))} />
      </div>
      <SpeedReach signal={signal} domain={domain} net={net} setSketch={setSketch} showOnPlan={showOnPlan} t={t}>
        {!readOnly && derived && !fits && (
          <Button size="small" onClick={() => set(withSpeedSignalSpeed(signal, derived.speed))}>{t('take_speed', { speed: Number(speedSymbol(derived.speed)) * 10 })}</Button>
        )}
      </SpeedReach>
    </Section>
  )
}

/**
 * The Zs 3 a signal shows for the train routes from it — the speed range each
 * of them begins with —, its figure chosen here route by route as an Lf 7's
 * is; a route without one has a range without a speed, which is said.
 */
function Zs3Section({ signal, domain, ops, select, readOnly, t }) {
  const { signals, routes } = domain.collections
  const fromHere = routes.filter(r => r.startSignalId === signal.id && r.kind === 'train')
  const routeName = (r) => r.name ?? `${labelOf('signals', signal, t)} → ${labelOf('signals', signals.find(s => s.id === r.targetSignalId), t)}`
  const unsignalled = fromHere.filter(r => signallingSpeedOf(domain, r) == null)
  return (
    <Section title={t('zs3_at_signal')}>
      <p className="ui-muted">{t('zs3_at_signal_hint')}</p>
      {fromHere.length ? fromHere.map(r => (
        <div key={r.id} className="ui-row tool-speed-row">
          <button type="button" className="ui-link ui-grow" onClick={() => select({ collection: 'routes', id: r.id })}>{routeName(r)}</button>
          <SpeedFigureField aspect="Zs_3" label={t('zs3_shown')} speed={signallingSpeedOf(domain, r)} readOnly={readOnly} t={t}
            onChange={(v) => ops.replaceAll(withRouteSpeed(domain, r, v))} />
        </div>
      )) : <p className="ui-muted">{t('zs3_none')}</p>}
      {unsignalled.length > 0 && <p className="ui-message ui-message-warn">{t('zs3_missing', { n: unsignalled.length })}</p>}
    </Section>
  )
}
