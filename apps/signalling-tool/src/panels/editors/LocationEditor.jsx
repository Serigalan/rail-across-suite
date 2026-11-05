import { Button, Checkbox, NumberField, Option, Options, Section, SelectField, TextField } from '@rail-across/ui'
import { SIGNAL_CATALOGUE } from '@rail-across/core/signalling'
import { labelOf, logicalId, newLogical } from '../../model/domain.js'
import { FUNCTION, MOUNTING, VIRTUAL_FUNCTION, logicalSignalWord, optionsOf, word } from '../../labels.js'
import { SignalIcon } from '../../icons.jsx'
import { EditorHead, LocationFields, ObjectFindings, PlaneLine } from '../common.jsx'
import FramesEditor from '../frames.jsx'
import { FOUNDATION_OFFSET, ON_FOUNDATION, foundationPoint, sideAlong, speedWhereText, trackName } from '../../model/places.js'
import { isSpeedSignal, speedSignalElementSpeed, speedSignalSpeed, speedSymbol, withSpeedSignalSpeed } from '../../model/routeSpeeds.js'
import { stationName } from '../../model/stations.js'

/**
 * A signal mast — the physical signal: where it stands, on what mounting,
 * from how far it is seen, the station it belongs to or the open line, the
 * boards it carries of no system, and its logical signals, each opened in an
 * editor of its own; a virtual signal has its function instead. On a gantry
 * or a bracket its location is its screen's, and its foundation has its own.
 * A speed signal Lf 7 says what the elements ahead of it allow.
 */
export default function LocationEditor({ signal, domain, net, core, findings, onChange, onDelete, onPlace, select, readOnly, t, language }) {
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
              options={optionsOf(FUNCTION, language)} onChange={(v) => set({ function: v ?? undefined })} />
          )}
          <SelectField compact label={t('mounting')} value={signal.mounting ?? null} empty={t('none')} disabled={readOnly}
            options={optionsOf(MOUNTING, language)} onChange={(v) => set({
              mounting: v ?? undefined,
              // A gantry or a bracket stands on a foundation, on the side the signal stood; nothing else has one.
              foundation: ON_FOUNDATION.has(v) ? signal.foundation ?? { side: sideAlong(signal.at, signal.at?.side) ?? 'right', offset: FOUNDATION_OFFSET } : undefined,
            })} />
          <SelectField compact wide label={t('station_of_signal')} value={signal.operationalPointId ?? null} empty={t('open_line')} disabled={readOnly}
            options={(core?.collections.operationalPoints ?? []).map(p => ({ value: p.id, label: stationName(p) }))
              .sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true }))}
            onChange={(v) => set({ operationalPointId: v ?? undefined })} />
          <div className="ui-check-cell"><Checkbox label={t('both_directions')} checked={signal.bothDirections} disabled={readOnly} onChange={(v) => set({ bothDirections: v || undefined })} /></div>
        </>}
      </div>

      {isSpeedSignal(signal) && <SpeedSignalSection signal={signal} domain={domain} net={net} set={onChange} readOnly={readOnly} t={t} />}

      {signal.kind === 'real' && (
        <Section title={t('logical_signals')}>
          <p className="ui-muted">{t('logical_signals_hint')}</p>
          <Options>
            {logical.map(l => (
              <Option key={l.system} icon={<SignalIcon />} end={l.function ? word(FUNCTION, l.function, language) : null}
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
              onChange={(v) => set({ foundation: { ...signal.foundation, side: v ?? 'right' } })} />
            <NumberField compact label={t('offset')} value={signal.foundation.offset} min={0} disabled={readOnly}
              onChange={(v) => set({ foundation: { ...signal.foundation, offset: v ?? FOUNDATION_OFFSET } })} />
            <PlaneLine point={foundationPoint(net, signal)} t={t} />
          </div>
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

/** What an Lf 7 shows, and what the elements from it to the next main signal or Lf 7 allow. */
function SpeedSignalSection({ signal, domain, net, set, readOnly, t }) {
  const shown = speedSignalSpeed(signal)
  const derived = speedSignalElementSpeed(net, domain.collections.signals, signal)
  const fits = derived && speedSymbol(derived.speed) === String((shown ?? 0) / 10)
  return (
    <Section title={t('speed_signal_section')}>
      <p>{t('speed_signal_shows', { speed: shown != null ? `${shown} km/h` : '—' })}</p>
      {derived && (
        <div className="ui-row">
          <span className="ui-grow ui-muted">{t('speed_signal_elements', { speed: derived.speed, where: speedWhereText(net, derived.where) })}</span>
          {!readOnly && !fits && (
            <Button size="small" onClick={() => set(withSpeedSignalSpeed(signal, derived.speed))}>{t('take_speed', { speed: Number(speedSymbol(derived.speed)) * 10 })}</Button>
          )}
        </div>
      )}
    </Section>
  )
}
