import { Badge, Button, Checkbox, Panel, PlusIcon, Section, SelectField, SelectList } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'

/**
 * Flank protection: per switch element and the way it is run over, what
 * protects it — a signal kept at stop, a switch or derailer held in a
 * position, or nothing (waived) — and the sections that must be clear.
 */
export default function FlankPanel({ domain, findings, selection, select, ops, readOnly, t }) {
  const { flankProtection: list, switchElements, signals, trackSections } = domain.collections
  const el = (id) => switchElements.find(e => e.id === id)
  const chosen = selection?.collection === 'flankProtection' ? list.find(f => f.id === selection.id) : null
  const bad = new Set(findings.errors.filter(f => f.collection === 'flankProtection').map(f => f.id))
  const viaText = (v) => (v === 'tip' ? t('tip') : t(v))
  const byText = (p) => (p.signalId ? labelOf('signals', signals.find(s => s.id === p.signalId), t)
    : p.switchElementId ? `${labelOf('switchElements', el(p.switchElementId), t)}${p.position ? ` ${t(p.position)}` : ''}` : t('waived'))
  const items = list.map(f => ({
    id: f.id, title: `${labelOf('switchElements', el(f.requester.switchElementId), t)} · ${viaText(f.requester.via)}`,
    sub: `${t('protection')}: ${byText(f.protection)}`, end: bad.has(f.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => a.title.localeCompare(b.title, 'de', { numeric: true }))
  const elementOptions = switchElements.map(e => ({ value: e.id, label: labelOf('switchElements', e, t) }))
  const signalOptions = signals.filter(s => s.kind === 'real' && s.frames?.some(f => f.kind === 'screen')).map(s => ({ value: s.id, label: labelOf('signals', s, t) }))
  const set = (patch) => ops.update('flankProtection', { ...chosen, ...patch })
  const how = chosen ? (chosen.protection.signalId ? 'signal' : chosen.protection.switchElementId ? 'element' : 'waived') : null

  return (
    <Panel title={t('section_flank')} intro={t('section_flank_intro')}
      actions={!readOnly && (
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!switchElements.length}
          onClick={() => ops.add('flankProtection', { id: newId(), requester: { switchElementId: switchElements[0].id, via: 'left' }, protection: { waived: true } })}>
          {t('new_flank')}
        </Button>
      )}>
      <SelectList maxHeight="34vh" label={t('section_flank')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'flankProtection', id })} empty={t('empty_list')} />
      {chosen && (
        <div className="tool-editor" key={chosen.id}>
          <EditorHead title={items.find(i => i.id === chosen.id)?.title} readOnly={readOnly} t={t} onDelete={() => ops.remove('flankProtection', chosen.id)} />
          <ObjectFindings findings={findings} collection="flankProtection" id={chosen.id} t={t} />
          <div className="ui-fields">
            <SelectField compact label={t('requester')} value={chosen.requester.switchElementId} options={elementOptions} disabled={readOnly}
              onChange={(v) => set({ requester: { ...chosen.requester, switchElementId: v } })} />
            <SelectField compact label={t('via')} value={chosen.requester.via} disabled={readOnly}
              options={['left', 'right', 'tip'].map(v => ({ value: v, label: viaText(v) }))} onChange={(v) => set({ requester: { ...chosen.requester, via: v } })} />
            <SelectField compact wide label={t('protection')} value={how} disabled={readOnly}
              options={[{ value: 'signal', label: t('by_signal') }, { value: 'element', label: t('by_element') }, { value: 'waived', label: t('waived') }]}
              onChange={(v) => set({ protection: v === 'signal' ? { signalId: signalOptions[0]?.value } : v === 'element' ? { switchElementId: elementOptions.find(o => o.value !== chosen.requester.switchElementId)?.value, position: 'left' } : { waived: true } })} />
            {how === 'signal' && <>
              <SelectField compact label={t('section_signals')} value={chosen.protection.signalId} options={signalOptions} disabled={readOnly}
                onChange={(v) => set({ protection: { ...chosen.protection, signalId: v } })} />
              <div className="ui-check-cell"><Checkbox label={t('target_lock')} checked={chosen.protection.targetLock} disabled={readOnly}
                onChange={(v) => set({ protection: { ...chosen.protection, targetLock: v || undefined } })} /></div>
            </>}
            {how === 'element' && <>
              <SelectField compact label={t('section_elements')} value={chosen.protection.switchElementId ?? null} options={elementOptions} disabled={readOnly}
                onChange={(v) => set({ protection: { ...chosen.protection, switchElementId: v } })} />
              {!el(chosen.protection.switchElementId)?.derailer && (
                <SelectField compact label={t('preferred')} value={chosen.protection.position ?? null} empty={t('none')} disabled={readOnly}
                  options={[{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]}
                  onChange={(v) => set({ protection: { ...chosen.protection, position: v ?? undefined } })} />
              )}
            </>}
          </div>
          <Section title={t('clear_sections')}>
            <CheckList readOnly={readOnly} value={chosen.clearSections} onChange={(v) => set({ clearSections: v.length ? v : undefined })}
              options={trackSections.map(s => ({ value: s.id, label: labelOf('trackSections', s, t) }))
                .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))} />
          </Section>
        </div>
      )}
    </Panel>
  )
}
