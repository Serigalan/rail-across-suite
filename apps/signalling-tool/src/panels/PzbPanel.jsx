import { Badge, Button, Checkbox, NumberField, Panel, PlusIcon, Section, SelectField, SelectList } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { FREQUENCY, optionsOf, word } from '../labels.js'
import { EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { locationFrom, trackName } from '../model/places.js'

/**
 * The PZB track elements: magnets of 500, 1000 and 2000 Hz and speed checks,
 * where they act, the signal they belong to, how they act. Where they must
 * lie to their signal is not checked yet (Ril 819, Ril 483).
 */
export default function PzbPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const { pzbElements: list, signals } = domain.collections
  const chosen = selection?.collection === 'pzbElements' ? list.find(p => p.id === selection.id) : null
  const bad = new Set(findings.errors.filter(f => f.collection === 'pzbElements').map(f => f.id))
  const sigName = (id) => (id ? labelOf('signals', signals.find(s => s.id === id), t) : t('none'))
  const items = list.map(p => ({
    id: p.id, title: `${word(FREQUENCY, p.frequency, language)}${p.kind === 'speedCheck' ? ` · ${t('speed_check')}` : ''}`,
    sub: `${sigName(p.signalId)} · ${p.at?.anchor === 'station' ? `${trackName(net, p.at.trackId)} · ${Math.round(p.at.station)} m` : t('orphan')}`,
    end: bad.has(p.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => a.sub.localeCompare(b.sub, 'de', { numeric: true }))
  const signalOptions = signals.filter(s => s.kind === 'real').map(s => ({ value: s.id, label: labelOf('signals', s, t) }))
    .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))
  const set = (patch) => ops.update('pzbElements', { ...chosen, ...patch })

  return (
    <Panel title={t('section_pzb')} intro={t('section_pzb_intro')}
      actions={!readOnly && (
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => setMode({ kind: 'place', apply: (hit) => ops.add('pzbElements', { id: newId(), at: locationFrom(hit), frequency: '1000', kind: 'magnet', effect: 'switchedBySignal' }) })}>
          {t('new_pzb')}
        </Button>
      )}>
      <SelectList maxHeight="34vh" label={t('section_pzb')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'pzbElements', id })} empty={t('empty_list')} />
      {chosen && (
        <div className="tool-editor" key={chosen.id}>
          <EditorHead title={items.find(i => i.id === chosen.id)?.title} readOnly={readOnly} t={t} onDelete={() => ops.remove('pzbElements', chosen.id)} />
          <ObjectFindings findings={findings} collection="pzbElements" id={chosen.id} t={t} />
          <div className="ui-fields">
            <SelectField compact label={t('frequency')} value={chosen.frequency} options={optionsOf(FREQUENCY, language)} disabled={readOnly} onChange={(v) => set({ frequency: v })} />
            <SelectField compact label={t('kind')} value={chosen.kind} disabled={readOnly}
              options={[{ value: 'magnet', label: t('magnet') }, { value: 'speedCheck', label: t('speed_check') }]}
              onChange={(v) => set(v === 'speedCheck' ? { kind: v, speedCheck: chosen.speedCheck ?? { testSpeed: 0, measuringLength: 0 } } : { kind: v, speedCheck: undefined })} />
            <SelectField compact wide label={t('belongs_to')} value={chosen.signalId ?? null} empty={t('none')} options={signalOptions} disabled={readOnly} onChange={(v) => set({ signalId: v ?? undefined })} />
            <SelectField compact wide label={t('effect')} value={chosen.effect} disabled={readOnly}
              options={[{ value: 'permanent', label: t('permanent') }, { value: 'switchedBySignal', label: t('switched_by_signal') }, { value: 'other', label: t('other') }]}
              onChange={(v) => set({ effect: v })} />
            <div className="ui-wide"><Checkbox label={t('ina')} checked={chosen.ina} disabled={readOnly} onChange={(v) => set({ ina: v || undefined })} /></div>
            {chosen.kind === 'speedCheck' && <>
              <NumberField compact label={t('test_speed')} value={chosen.speedCheck?.testSpeed ?? null} min={0} disabled={readOnly}
                onChange={(v) => set({ speedCheck: { ...chosen.speedCheck, testSpeed: v ?? 0 } })} />
              <NumberField compact label={t('measuring_length')} value={chosen.speedCheck?.measuringLength ?? null} min={0} disabled={readOnly}
                onChange={(v) => set({ speedCheck: { ...chosen.speedCheck, measuringLength: v ?? 0 } })} />
            </>}
          </div>
          <Section title={t('location')}>
            <LocationFields loc={chosen.at} net={net} t={t} readOnly={readOnly} onChange={(at) => set({ at })}
              onPlace={() => setMode({ kind: 'place', apply: (hit) => ops.update('pzbElements', { ...chosen, at: locationFrom(hit, { direction: chosen.at?.direction ?? 'up' }) }) })} />
          </Section>
        </div>
      )}
    </Panel>
  )
}
