import { useMemo, useState } from 'react'
import {
  Badge, Button, Checkbox, NumberField, Panel, PlusIcon, Section, SelectField, SelectList, Tabs, TextField,
} from '@rail-across/ui'
import { SIGNAL_CATALOGUE } from '@rail-across/core/signalling'
import { labelOf, logicalId, newId, newLogical, newSignal } from '../model/domain.js'
import { FUNCTION, MOUNTING, VIRTUAL_FUNCTION, logicalSignalWord, optionsOf, word } from '../labels.js'
import { signalTone, TONE_FILL } from '../plan/symbols.js'
import { EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import FramesEditor from './frames.jsx'
import { OFFSET, locationFrom, placing, trackName } from '../model/places.js'

const FILTERS = ['all', 'main', 'distant', 'shunting', 'board', 'virtual']

/**
 * The places of the signals — the physical signals: where each stands, on
 * what mounting, from how far it is seen, the boards it carries of no system,
 * and which logical signals it has (the Signale section edits those). A new
 * place or board is set by a click on the plan.
 */
export default function LocationsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const [filter, setFilter] = useState('all')
  const signals = domain.collections.signals
  const chosen = selection?.collection === 'signals' ? signals.find(s => s.id === selection.id) : null
  const bad = useMemo(() => new Set(findings.errors.filter(f => f.collection === 'signals').map(f => f.id)), [findings])

  const items = signals
    .filter(s => filter === 'all' || signalTone(s) === filter || (filter === 'board' && signalTone(s) === 'marker'))
    .map(s => ({
      id: s.id,
      title: labelOf('signals', s, t),
      sub: [s.kind === 'virtual' ? word(VIRTUAL_FUNCTION, s.virtualFunction, language)
        : (s.logicalSignals ?? []).map(l => logicalSignalWord(l, language)).join(' + ') || t('no_logical_signal'),
      s.at?.anchor === 'station' ? `${trackName(net, s.at.trackId)} · ${Math.round(s.at.station)} m` : t('orphan')].join(' · '),
      mark: <svg width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="5" fill={TONE_FILL[signalTone(s)] === 'none' ? 'transparent' : TONE_FILL[signalTone(s)]} stroke="currentColor" /></svg>,
      end: bad.has(s.id) ? <Badge tone="danger">!</Badge> : null,
    }))
    .sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))

  const place = (make) => setMode(placing('signals', make, (s) => ops.add('signals', s)))

  return (
    <Panel title={t('section_locations')} intro={t('section_locations_intro')}
      actions={!readOnly && <>
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => place(hit => newSignal({ at: locationFrom(hit, { offset: OFFSET.signals }), bare: true }))}>{t('new_location')}</Button>
        <Button size="small" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => place(hit => ({ id: newId(), kind: 'real', function: 'standaloneIndicator', mounting: 'mast', at: locationFrom(hit, { offset: OFFSET.signals }), frames: [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }] }))}>
          {t('new_board')}
        </Button>
      </>}>
      <Tabs label={t('kind')} value={filter} onChange={setFilter}
        options={FILTERS.map(f => ({ value: f, label: f === 'all' ? t('signals_count', { n: signals.length }) : t(`tone_${f}`) }))} />
      <SelectList maxHeight="34vh" label={t('section_locations')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'signals', id })} empty={t('empty_list')} />
      {chosen && (
        <LocationEditor key={chosen.id} signal={chosen} net={net} findings={findings} readOnly={readOnly} t={t} language={language} select={select}
          onChange={(s) => ops.update('signals', s)} onDelete={() => ops.remove('signals', chosen.id)}
          onPlace={() => setMode(placing('signals', (hit) => ({ ...chosen, at: locationFrom(hit, { offset: chosen.at?.offset ?? OFFSET.signals }) }), (s) => ops.update('signals', s)))} />
      )}
    </Panel>
  )
}

function LocationEditor({ signal, net, findings, onChange, onDelete, onPlace, select, readOnly, t, language }) {
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
        <Section title={t('logical_signals')}>
          <p className="ui-muted">{t('logical_signals_hint')}</p>
          {logical.map(l => (
            <div key={l.system} className="ui-row">
              <button type="button" className="ui-link ui-grow" onClick={() => select({ collection: 'logicalSignals', id: logicalId(signal.id, l.system) })}>
                {logicalSignalWord(l, language)}{l.function ? ` · ${word(FUNCTION, l.function, language)}` : ''}
              </button>
            </div>
          ))}
          {!readOnly && free.length > 0 && (
            <select className="ui-input" aria-label={t('add_logical_signal')} value=""
              onChange={(e) => { if (e.target.value) addLogical(e.target.value) }}>
              <option value="">+ {t('add_logical_signal')}</option>
              {free.map(sys => <option key={sys} value={sys}>{logicalSignalWord({ system: sys }, language)}</option>)}
            </select>
          )}
        </Section>
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
