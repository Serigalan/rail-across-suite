import { Badge, Button, Checkbox, Panel, PlusIcon, Section, SelectField, SelectList, TextField } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { OPERATION, optionsOf, word } from '../labels.js'
import { EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { locationFrom, placing } from '../model/places.js'

/**
 * The switch elements: the switches of the net as the interlocking knows
 * them — a switch of the core without one is offered to get one —, and the
 * derailers, which stand at a place of their own.
 */
export default function ElementsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const elements = domain.collections.switchElements
  const signals = domain.collections.signals
  const withElement = new Set(elements.map(e => e.switchId).filter(Boolean))
  const bare = net.switches.filter(sw => sw.kind !== 'link' && !withElement.has(sw.switchId))
  const chosen = selection?.collection === 'switchElements' ? elements.find(e => e.id === selection.id)
    : selection?.collection === 'switches' ? elements.find(e => e.switchId === selection.id) : null
  const chosenBare = selection?.collection === 'switches' && !chosen ? net.switches.find(sw => sw.switchId === selection.id) : null
  const switchName = (id) => { const sw = net.switches.find(x => x.switchId === id); return sw?.name ?? sw?.label ?? id?.slice(0, 8) }
  const markers = signals.filter(s => (s.frames ?? []).some(f => f.aspects.some(a => a.aspect === 'Ra_12')))
  const bad = new Set(findings.errors.filter(f => f.collection === 'switchElements').map(f => f.id))

  const items = elements.map(e => ({
    id: e.id, title: labelOf('switchElements', e, t),
    sub: [e.derailer ? t('derailer') : `${t('switch')} ${switchName(e.switchId)}`, word(OPERATION, e.operation, language)].join(' · '),
    end: bad.has(e.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))

  const newElement = (sw) => ops.add('switchElements', { id: newId(), name: sw.name ?? undefined, switchId: sw.switchId, operation: 'electricRemote' })
  const set = (patch) => ops.update('switchElements', { ...chosen, ...patch })

  return (
    <Panel title={t('section_elements')} intro={t('section_elements_intro')}
      actions={!readOnly && (
        <Button size="small" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => setMode(placing('switchElements', (hit) => ({ id: newId(), operation: 'electricRemote', derailer: { at: locationFrom(hit), preferredPosition: 'applied' } }), (e) => ops.add('switchElements', e)))}>
          {t('derailer')}
        </Button>
      )}>
      <SelectList maxHeight="34vh" label={t('section_elements')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'switchElements', id })} empty={t('empty_list')} />
      {chosenBare && !readOnly && (
        <div className="tool-editor">
          <EditorHead title={`${t('switch')} ${switchName(chosenBare.switchId)}`} t={t} readOnly />
          <Button size="small" variant="primary" icon={<PlusIcon size={14} />} onClick={() => newElement(chosenBare)}>{t('new_element')}</Button>
        </div>
      )}
      {chosen && (
        <div className="tool-editor" key={chosen.id}>
          <EditorHead title={labelOf('switchElements', chosen, t)} readOnly={readOnly} t={t} onDelete={() => ops.remove('switchElements', chosen.id)} />
          <ObjectFindings findings={findings} collection="switchElements" id={chosen.id} t={t} />
          <div className="ui-fields">
            <TextField compact label={t('name')} value={chosen.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
            <TextField compact label={t('area')} value={chosen.areaNumber ?? ''} disabled={readOnly} onChange={(v) => set({ areaNumber: v || undefined })} />
            <SelectField compact wide label={t('operation')} value={chosen.operation} options={optionsOf(OPERATION, language)} disabled={readOnly} onChange={(v) => set({ operation: v })} />
            {chosen.derailer ? (
              <SelectField compact wide label={t('preferred')} value={chosen.derailer.preferredPosition ?? null} empty={t('none')} disabled={readOnly}
                options={[{ value: 'applied', label: 'aufgelegt' }, { value: 'removed', label: 'abgelegt' }]}
                onChange={(v) => set({ derailer: { ...chosen.derailer, preferredPosition: v ?? undefined } })} />
            ) : <>
              <SelectField compact label={t('switch')} value={chosen.switchId} disabled={readOnly}
                options={net.switches.filter(sw => sw.kind !== 'link').map(sw => ({ value: sw.switchId, label: switchName(sw.switchId) }))}
                onChange={(v) => set({ switchId: v })} />
              <SelectField compact label={t('preferred')} value={chosen.preferredPosition ?? null} empty={t('none')} disabled={readOnly}
                options={[{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]} onChange={(v) => set({ preferredPosition: v ?? undefined })} />
              <SelectField compact wide label={t('clearance_marker')} value={chosen.clearanceMarkerId ?? null} empty={t('none')} disabled={readOnly}
                options={markers.map(s => ({ value: s.id, label: labelOf('signals', s, t) }))} onChange={(v) => set({ clearanceMarkerId: v ?? undefined })} />
              <div className="ui-wide"><Checkbox label={t('trailable')} checked={chosen.trailable} disabled={readOnly} onChange={(v) => set({ trailable: v || undefined })} /></div>
            </>}
          </div>
          {chosen.derailer && (
            <Section title={t('location')}>
              <LocationFields loc={chosen.derailer.at} net={net} t={t} readOnly={readOnly}
                onChange={(at) => set({ derailer: { ...chosen.derailer, at } })}
                onPlace={() => setMode(placing('switchElements', (hit) => ({ ...chosen, derailer: { ...chosen.derailer, at: locationFrom(hit, { offset: chosen.derailer.at?.offset }) } }), (e) => ops.update('switchElements', e)))} />
            </Section>
          )}
        </div>
      )}
      {!readOnly && bare.length > 0 && (
        <Section title={`${t('switches_without')} (${bare.length})`}>
          <div className="tool-chips">
            {bare.map(sw => <Button key={sw.switchId} size="small" icon={<PlusIcon size={12} />} onClick={() => newElement(sw)}>{switchName(sw.switchId)}</Button>)}
          </div>
        </Section>
      )}
    </Panel>
  )
}
