import { useMemo, useState } from 'react'
import {
  Badge, Button, Checkbox, NumberField, Panel, PlusIcon, Section, SelectField, SelectList, Tabs, TextField,
} from '@rail-across/ui'
import { SIGNAL_CATALOGUE, aspectOf, signalKindOf } from '@rail-across/core/signalling'
import { newSignal, newId, withKind, labelOf } from '../model/domain.js'
import { FUNCTION, MOUNTING, SYSTEM, TYPE, VIRTUAL_FUNCTION, optionsOf, word } from '../labels.js'
import { signalTone, TONE_FILL } from '../plan/symbols.js'
import { EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { locationFrom, trackName } from '../model/places.js'

const FILTERS = ['all', 'main', 'distant', 'shunting', 'board', 'virtual']
const KS_TYPES = Object.keys(SIGNAL_CATALOGUE.systeme.ks.signalarten)

/**
 * The signals: a list to pick from, filtered by what they are, and the
 * editor of the one chosen — where it stands, what it is, what its frames
 * show. A new signal or board is placed by a click on the plan.
 */
export default function SignalsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const [filter, setFilter] = useState('all')
  const signals = domain.collections.signals
  const chosen = selection?.collection === 'signals' ? signals.find(s => s.id === selection.id) : null
  const bad = useMemo(() => new Set(findings.errors.filter(f => f.collection === 'signals').map(f => f.id)), [findings])

  const items = signals
    .filter(s => filter === 'all' || signalTone(s) === filter || (filter === 'board' && signalTone(s) === 'marker'))
    .map(s => ({
      id: s.id,
      title: labelOf('signals', s, t),
      sub: [s.kind === 'virtual' ? word(VIRTUAL_FUNCTION, s.virtualFunction, language) : word(TYPE, s.type, language) || word(FUNCTION, s.function, language),
        s.at?.anchor === 'station' ? `${trackName(net, s.at.trackId)} · ${Math.round(s.at.station)} m` : t('orphan')].filter(Boolean).join(' · '),
      mark: <svg width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="5" fill={TONE_FILL[signalTone(s)] === 'none' ? 'transparent' : TONE_FILL[signalTone(s)]} stroke="currentColor" /></svg>,
      end: bad.has(s.id) ? <Badge tone="danger">!</Badge> : null,
    }))
    .sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))

  const place = (make) => setMode({ kind: 'place', apply: (hit) => ops.add('signals', make(hit)) })

  return (
    <Panel title={t('section_signals')} intro={t('section_signals_intro')}
      actions={!readOnly && <>
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => place(hit => newSignal({ at: locationFrom(hit) }))}>{t('new_signal')}</Button>
        <Button size="small" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => place(hit => ({ id: newId(), kind: 'real', function: 'other', mounting: 'mast', at: locationFrom(hit), frames: [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }] }))}>
          {t('new_board')}
        </Button>
      </>}>
      <Tabs label={t('kind')} value={filter} onChange={setFilter}
        options={FILTERS.map(f => ({ value: f, label: f === 'all' ? t('signals_count', { n: signals.length }) : t(`tone_${f}`) }))} />
      <SelectList maxHeight="34vh" label={t('section_signals')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'signals', id })} empty={t('empty_list')} />
      {chosen && (
        <SignalEditor key={chosen.id} signal={chosen} net={net} findings={findings} readOnly={readOnly} t={t} language={language}
          onChange={(s) => ops.update('signals', s)} onDelete={() => ops.remove('signals', chosen.id)}
          onPlace={() => setMode({ kind: 'place', apply: (hit) => ops.update('signals', { ...chosen, at: locationFrom(hit, { direction: chosen.at?.direction ?? 'up' }) }) })} />
      )}
    </Panel>
  )
}

function SignalEditor({ signal, net, findings, onChange, onDelete, onPlace, readOnly, t, language }) {
  const set = (patch) => onChange({ ...signal, ...patch })
  const kind = signalKindOf(signal.system, signal.type)
  return (
    <div className="tool-editor">
      <EditorHead title={labelOf('signals', signal, t)} onDelete={onDelete} readOnly={readOnly} t={t} />
      <ObjectFindings findings={findings} collection="signals" id={signal.id} t={t} />
      <div className="ui-fields">
        <TextField compact label={t('name')} value={signal.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <TextField compact label={t('area')} value={signal.areaNumber ?? ''} disabled={readOnly} onChange={(v) => set({ areaNumber: v || undefined })} />
        <SelectField compact wide label={t('kind')} value={signal.kind} disabled={readOnly}
          options={[{ value: 'real', label: t('real') }, { value: 'virtual', label: t('virtual') }]}
          onChange={(v) => set(v === 'virtual'
            ? { kind: 'virtual', frames: undefined, type: undefined, system: undefined, function: undefined, mounting: undefined, virtualFunction: signal.virtualFunction ?? 'trainTargetLine' }
            : { kind: 'real', virtualFunction: undefined, frames: [], function: 'other' })} />
        {signal.kind === 'virtual' ? (
          <SelectField compact wide label={t('virtual_function')} value={signal.virtualFunction ?? null} disabled={readOnly}
            options={optionsOf(VIRTUAL_FUNCTION, language)} onChange={(v) => set({ virtualFunction: v ?? undefined })} />
        ) : <>
          <SelectField compact wide label={t('function')} value={signal.function ?? null} empty={t('none')} disabled={readOnly}
            options={optionsOf(FUNCTION, language)} onChange={(v) => set({ function: v ?? undefined })} />
          <SelectField compact label={t('system')} value={signal.system ?? null} empty={t('none')} disabled={readOnly}
            options={optionsOf(SYSTEM, language).filter(o => o.value === 'ks' || o.value === signal.system)}
            onChange={(v) => onChange(v ? withKind(signal, v, signal.type ?? 'main') : { ...signal, system: undefined })} />
          <SelectField compact label={t('type')} value={signal.type ?? null} empty={t('none')} disabled={readOnly}
            options={optionsOf(TYPE, language).filter(o => !signal.system || KS_TYPES.includes(o.value) || o.value === signal.type)}
            onChange={(v) => onChange(v && signal.system ? withKind(signal, signal.system, v) : { ...signal, type: v ?? undefined })} />
          <SelectField compact label={t('mounting')} value={signal.mounting ?? null} empty={t('none')} disabled={readOnly}
            options={optionsOf(MOUNTING, language)} onChange={(v) => set({ mounting: v ?? undefined })} />
          <div className="ui-check-cell"><Checkbox label={t('both_directions')} checked={signal.bothDirections} disabled={readOnly} onChange={(v) => set({ bothDirections: v || undefined })} /></div>
        </>}
      </div>

      <Section title={t('location')}>
        <LocationFields loc={signal.at} net={net} t={t} readOnly={readOnly} onPlace={onPlace} onChange={(at) => set({ at })} />
        {(signal.alsoAt ?? []).map((also, i) => (
          <p key={i} className="ui-muted">+ {trackName(net, also.trackId)} · {Math.round(also.station)} m</p>
        ))}
      </Section>

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
        <FramesEditor signal={signal} kind={kind} readOnly={readOnly} t={t} onChange={(frames) => set({ frames })} />
      </>}
    </div>
  )
}

/** The aspects that may stand on a frame of that kind, for a signal of that kind. */
function choicesFor(frameKind, kind) {
  const all = SIGNAL_CATALOGUE.begriffe.filter(b => !b.nur_dv && (!b.rahmen || b.rahmen.includes(frameKind)))
  if (frameKind === 'indicator' && kind) return all.filter(b => kind.zusatz.includes(b.id))
  return all
}

/**
 * The frames of a signal: its screen as a set of what it shows (what its kind
 * must show marked), each indicator and plate with its aspects and symbols.
 */
function FramesEditor({ signal, kind, onChange, readOnly, t }) {
  const frames = signal.frames ?? []
  const screenAt = frames.findIndex(f => f.kind === 'screen')
  const setFrame = (i, frame) => onChange(frames.map((f, j) => (j === i ? frame : f)))
  const removeFrame = (i) => onChange(frames.filter((_, j) => j !== i))
  const screenAspects = kind ? kind.schirm.erlaubt : []

  return (
    <Section title={t('frames')}>
      {kind && (
        <div className="tool-frame">
          <div className="tool-frame-head">{t('frame_screen')}</div>
          <div className="tool-checklist">
            {screenAspects.map(id => {
              const on = screenAt >= 0 && frames[screenAt].aspects.some(a => a.aspect === id)
              const must = kind.schirm.pflicht.includes(id)
              return (
                <Checkbox key={id} checked={on} disabled={readOnly || (must && on)}
                  label={<>{aspectOf(id)?.name ?? id}{must && <span className="tool-must"> · {t('must')}</span>}<span className="ui-muted tool-aspect-title"> {aspectOf(id)?.title}</span></>}
                  onChange={(v) => {
                    const screen = screenAt >= 0 ? frames[screenAt] : { kind: 'screen', aspects: [] }
                    const aspects = v ? [...screen.aspects, { aspect: id }] : screen.aspects.filter(a => a.aspect !== id)
                    if (screenAt >= 0) setFrame(screenAt, { ...screen, aspects })
                    else onChange([{ ...screen, aspects }, ...frames])
                  }} />
              )
            })}
          </div>
        </div>
      )}
      {frames.map((f, i) => ((kind && i === screenAt) ? null : (
        <FrameEditor key={i} frame={f} kind={kind} readOnly={readOnly} t={t} onChange={(fr) => setFrame(i, fr)} onRemove={() => removeFrame(i)} />
      )))}
      {!readOnly && (
        <div className="ui-row">
          <Button size="small" icon={<PlusIcon size={14} />} onClick={() => onChange([...frames, { kind: 'indicator', aspects: [] }])}>{t('add_indicator')}</Button>
          <Button size="small" icon={<PlusIcon size={14} />} onClick={() => onChange([...frames, { kind: 'plate', aspects: [] }])}>{t('add_plate')}</Button>
          {!frames.some(f => f.kind === 'nameplate') && (
            <Button size="small" icon={<PlusIcon size={14} />} onClick={() => onChange([...frames, { kind: 'nameplate', aspects: [] }])}>{t('add_nameplate')}</Button>
          )}
        </div>
      )}
    </Section>
  )
}

function FrameEditor({ frame, kind, onChange, onRemove, readOnly, t }) {
  const [adding, setAdding] = useState('')
  const choices = choicesFor(frame.kind, kind)
  const setAspect = (i, a) => onChange({ ...frame, aspects: frame.aspects.map((x, j) => (j === i ? a : x)) })
  return (
    <div className="tool-frame">
      <div className="tool-frame-head">
        <span className="ui-grow">{t(`frame_${frame.kind}`)}</span>
        {!readOnly && <button type="button" className="ui-link" onClick={onRemove}>{t('remove')}</button>}
      </div>
      {frame.aspects.map((a, i) => {
        const def = aspectOf(a.aspect)
        return (
          <div key={i} className="tool-aspect-row">
            <span className="ui-grow" title={def?.title}>{def?.name ?? a.aspect}<span className="ui-muted tool-aspect-title"> {def?.title}</span></span>
            {(def?.symbole || def?.symbol_frei) && (def?.symbole ? (
              <select className="ui-input tool-symbol" aria-label={t('symbol')} value={a.symbol ?? ''} disabled={readOnly}
                onChange={(e) => setAspect(i, { ...a, symbol: e.target.value || undefined })}>
                <option value="">{t('symbol')}</option>
                {def.symbole.map(s => <option key={s} value={s}>{def.symbol_bedeutung === 'speed' ? `${s} (${Number(s) * 10} km/h)` : s}</option>)}
              </select>
            ) : (
              <input className="ui-input tool-symbol" aria-label={t('symbol')} value={a.symbol ?? ''} disabled={readOnly}
                onChange={(e) => setAspect(i, { ...a, symbol: e.target.value || undefined })} />
            ))}
            {!readOnly && <button type="button" className="ui-link" aria-label={t('remove')} onClick={() => onChange({ ...frame, aspects: frame.aspects.filter((_, j) => j !== i) })}>×</button>}
          </div>
        )
      })}
      {!readOnly && frame.kind !== 'nameplate' && (
        <select className="ui-input" aria-label={t('aspect')} value={adding}
          onChange={(e) => { const v = e.target.value; setAdding(''); if (v) onChange({ ...frame, aspects: [...frame.aspects, { aspect: v }] }) }}>
          <option value="">+ {t('aspect')}</option>
          {choices.map(b => <option key={b.id} value={b.id}>{b.name}{b.title ? ` — ${b.title}` : ''}</option>)}
        </select>
      )}
    </div>
  )
}
