import { Badge, Button, Checkbox, Panel, PlusIcon, Section, SelectField, SelectList, TextField } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { OPERATION, optionsOf, word } from '../labels.js'
import { CheckList, EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { hasScreen } from '../model/domain.js'
import { locationFrom, placing } from '../model/places.js'

/**
 * The switches: the switch elements of the net as the interlocking knows
 * them — a switch of the core without one is offered to get one —, the
 * derailers, which stand at a place of their own, and with each the flank
 * protection it asks for in each position it is run over: a signal kept at
 * stop, a switch or derailer held in a position, or none (waived).
 */
export default function ElementsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const elements = domain.collections.switchElements
  const signals = domain.collections.signals
  const withElement = new Set(elements.map(e => e.switchId).filter(Boolean))
  const bare = net.switches.filter(sw => sw.kind !== 'link' && !withElement.has(sw.switchId))
  const chosen = selection?.collection === 'switchElements' ? elements.find(e => e.id === selection.id)
    : selection?.collection === 'switches' ? elements.find(e => e.switchId === selection.id)
      : selection?.collection === 'flankProtection' ? elements.find(e => e.id === domain.collections.flankProtection.find(f => f.id === selection.id)?.requester.switchElementId)
        : null
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
                options={[{ value: 'applied', label: t('applied') }, { value: 'removed', label: t('removed') }]}
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
          <FlankSection element={chosen} domain={domain} findings={findings} ops={ops} readOnly={readOnly} t={t}
            selected={selection?.collection === 'flankProtection' ? selection.id : null} />
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

/**
 * The flank protection a switch element asks for, one per way it is run over:
 * what gives it, and the sections that must be clear for it. A way without
 * one is offered to get one.
 */
function FlankSection({ element, domain, findings, ops, readOnly, t, selected }) {
  const { flankProtection, switchElements, signals, trackSections } = domain.collections
  const mine = flankProtection.filter(f => f.requester.switchElementId === element.id)
  const ways = element.derailer ? ['tip'] : ['left', 'right', 'tip']
  const missing = ways.filter(v => !mine.some(f => f.requester.via === v))
  const viaText = (v) => (v === 'tip' ? t('tip') : t(v))
  const others = switchElements.filter(e => e.id !== element.id).map(e => ({ value: e.id, label: labelOf('switchElements', e, t) }))
  const signalOptions = signals.filter(s => s.kind === 'real' && hasScreen(s)).map(s => ({ value: s.id, label: labelOf('signals', s, t) }))
    .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))
  return (
    <Section title={t('section_flank')}>
      <p className="ui-muted">{t('section_flank_intro')}</p>
      {mine.map(f => {
        const set = (patch) => ops.update('flankProtection', { ...f, ...patch })
        const how = f.protection.signalId ? 'signal' : f.protection.switchElementId ? 'element' : 'waived'
        const protecting = switchElements.find(e => e.id === f.protection.switchElementId)
        return (
          <div key={f.id} className={`tool-frame${selected === f.id ? ' tool-frame-chosen' : ''}`}>
            <div className="tool-frame-head">
              <span className="ui-grow">{t('via')} {viaText(f.requester.via)}</span>
              {!readOnly && <button type="button" className="ui-link" onClick={() => ops.remove('flankProtection', f.id)}>{t('remove')}</button>}
            </div>
            <ObjectFindings findings={findings} collection="flankProtection" id={f.id} t={t} />
            <div className="ui-fields">
              <SelectField compact wide label={t('protection')} value={how} disabled={readOnly}
                options={[{ value: 'signal', label: t('by_signal') }, { value: 'element', label: t('by_element') }, { value: 'waived', label: t('waived') }]}
                onChange={(v) => set({ protection: v === 'signal' ? { signalId: signalOptions[0]?.value } : v === 'element' ? { switchElementId: others[0]?.value, position: 'left' } : { waived: true } })} />
              {how === 'signal' && <>
                <SelectField compact label={t('signal')} value={f.protection.signalId ?? null} options={signalOptions} disabled={readOnly}
                  onChange={(v) => set({ protection: { ...f.protection, signalId: v } })} />
                <div className="ui-check-cell"><Checkbox label={t('target_lock')} checked={f.protection.targetLock} disabled={readOnly}
                  onChange={(v) => set({ protection: { ...f.protection, targetLock: v || undefined } })} /></div>
              </>}
              {how === 'element' && <>
                <SelectField compact label={t('switch')} value={f.protection.switchElementId ?? null} options={others} disabled={readOnly}
                  onChange={(v) => set({ protection: { ...f.protection, switchElementId: v } })} />
                {!protecting?.derailer && (
                  <SelectField compact label={t('preferred')} value={f.protection.position ?? null} empty={t('none')} disabled={readOnly}
                    options={[{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]}
                    onChange={(v) => set({ protection: { ...f.protection, position: v ?? undefined } })} />
                )}
              </>}
            </div>
            <CheckList readOnly={readOnly} value={f.clearSections} onChange={(v) => set({ clearSections: v.length ? v : undefined })}
              options={trackSections.map(x => ({ value: x.id, label: `${t('clear_sections')}: ${labelOf('trackSections', x, t)}` }))
                .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))} />
          </div>
        )
      })}
      {!readOnly && missing.length > 0 && (
        <div className="ui-row">
          {missing.map(v => (
            <Button key={v} size="small" icon={<PlusIcon size={12} />}
              onClick={() => ops.add('flankProtection', { id: newId(), requester: { switchElementId: element.id, via: v }, protection: { waived: true } })}>
              {t('via')} {viaText(v)}
            </Button>
          ))}
        </div>
      )}
    </Section>
  )
}
