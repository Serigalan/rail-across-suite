import { useState } from 'react'
import {
  Badge, Button, Checkbox, GroupedList, Option, Options, OptionsDivider, OptionsHeading, Panel, PlusIcon, Section, SelectField, TextField,
} from '@rail-across/ui'
import { newId, labelOf, hasScreen } from '../model/domain.js'
import { OPERATION, optionsOf, word } from '../labels.js'
import { DerailerIcon, SwitchIcon } from '../icons.jsx'
import { CheckList, Created, EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { locationFrom, placing } from '../model/places.js'

/**
 * The switches, with a menu as the Layout-Tool's panels: the switch elements
 * of the net as the interlocking knows them, listed under switches and
 * derailers; the switches of the core without one, each offered to get one;
 * a derailer set on the plan. A switch element opens in its editor — from
 * the list, the check or the plan —, with the flank protection it asks for
 * in each position it is run over: a signal kept at stop, a switch or
 * derailer held in a position, or none (waived).
 */
export default function ElementsPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t, language }) {
  const [page, setPage] = useState('menu')
  const [made, setMade] = useState(null)
  const [operation, setOperation] = useState('electricRemote')
  const [applied, setApplied] = useState('applied')
  const go = (next) => { setMade(null); setPage(next) }
  const back = { label: t('back'), onClick: () => go('menu') }
  const elements = domain.collections.switchElements
  const withElement = new Set(elements.map(e => e.switchId).filter(Boolean))
  const bare = net.switches.filter(sw => sw.kind !== 'link' && !withElement.has(sw.switchId))
  const chosen = selection?.collection === 'switchElements' ? elements.find(e => e.id === selection.id)
    : selection?.collection === 'switches' ? elements.find(e => e.switchId === selection.id)
      : selection?.collection === 'flankProtection' ? elements.find(e => e.id === domain.collections.flankProtection.find(f => f.id === selection.id)?.requester.switchElementId)
        : null
  const chosenBare = selection?.collection === 'switches' && !chosen ? net.switches.find(sw => sw.switchId === selection.id) : null
  const switchName = (id) => { const sw = net.switches.find(x => x.switchId === id); return sw?.name ?? sw?.label ?? id?.slice(0, 8) }
  const bad = new Set(findings.errors.filter(f => f.collection === 'switchElements').map(f => f.id))
  const newElement = (sw) => ({ id: newId(), name: sw.name ?? undefined, switchId: sw.switchId, operation: 'electricRemote' })
  const toPage = { label: t('back'), onClick: () => select(null) }

  if (chosen) {
    return (
      <Panel title={t(chosen.derailer ? 'object_derailer' : 'object_switchElements')} back={toPage}>
        <ElementEditor key={chosen.id} element={chosen} domain={domain} net={net} findings={findings} ops={ops} setMode={setMode} readOnly={readOnly} t={t} language={language}
          switchName={switchName} selected={selection?.collection === 'flankProtection' ? selection.id : null} />
      </Panel>
    )
  }
  if (chosenBare) {
    return (
      <Panel title={`${t('switch')} ${switchName(chosenBare.switchId)}`} intro={t('switch_without_hint')} back={toPage}>
        {!readOnly && (
          <div className="ui-row">
            <Button variant="primary" icon={<PlusIcon size={14} />} onClick={() => { const e = newElement(chosenBare); ops.add('switchElements', e); select({ collection: 'switchElements', id: e.id }) }}>
              {t('new_element')}
            </Button>
          </div>
        )}
      </Panel>
    )
  }

  if (page === 'list') {
    const item = (e) => ({
      id: e.id, label: labelOf('switchElements', e, t),
      sub: [e.derailer ? null : `${t('switch')} ${switchName(e.switchId)}`, word(OPERATION, e.operation, language)].filter(Boolean).join(' · '),
      end: bad.has(e.id) ? <Badge tone="danger">!</Badge> : null,
    })
    const byName = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })
    return (
      <Panel title={t('switch_elements')} intro={t('switch_elements_intro')} back={back}>
        <GroupedList label={t('switch_elements')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('empty_list')}
          groups={[
            { key: 'switches', title: t('group_switches'), items: elements.filter(e => !e.derailer).map(item).sort(byName) },
            { key: 'derailers', title: t('group_derailers'), items: elements.filter(e => e.derailer).map(item).sort(byName) },
          ]}
          onSelect={(id) => select({ collection: 'switchElements', id })} />
      </Panel>
    )
  }

  if (page === 'bare') {
    return (
      <Panel title={t('switches_without')} intro={t('switches_without_intro')} back={back}>
        {bare.length ? <>
          <div className="ui-row">
            <Button variant="primary" icon={<PlusIcon size={14} />} onClick={() => ops.replaceAll({
              ...domain, collections: { ...domain.collections, switchElements: [...elements, ...bare.map(newElement)] },
            })}>{t('create_all', { n: bare.length })}</Button>
          </div>
          <Options>
            {bare.map(sw => (
              <Option key={sw.switchId} icon={<SwitchIcon />} end={<PlusIcon size={14} />}
                onClick={() => { const e = newElement(sw); ops.add('switchElements', e); select({ collection: 'switchElements', id: e.id }) }}>
                {t('switch')} {switchName(sw.switchId)}
              </Option>
            ))}
          </Options>
        </> : <p className="ui-list-empty">{t('switches_without_none')}</p>}
      </Panel>
    )
  }

  if (page === 'derailer') {
    return (
      <Panel title={t('create_derailer')} intro={t('create_derailer_intro')} back={back}>
        <div className="ui-fields">
          <SelectField compact wide label={t('operation')} value={operation} options={optionsOf(OPERATION, language)} onChange={(v) => setOperation(v ?? 'electricRemote')} />
          <SelectField compact wide label={t('preferred')} value={applied} options={[{ value: 'applied', label: t('applied') }, { value: 'removed', label: t('removed') }]} onChange={setApplied} />
        </div>
        <div className="ui-row">
          <Button variant="primary" disabled={!net.tracks.length}
            onClick={() => setMode(placing('switchElements', (hit) => ({ id: newId(), operation, derailer: { at: locationFrom(hit), preferredPosition: applied } }), (e) => {
              ops.add('switchElements', e)
              setMade({ id: e.id, what: labelOf('switchElements', e, t) })
            }))}>
            {t('place_on_map')}
          </Button>
        </div>
        <Created what={made?.what} t={t} onEdit={() => select({ collection: 'switchElements', id: made.id })} />
      </Panel>
    )
  }

  return (
    <Panel title={t('section_elements')} intro={t('section_elements_intro')}>
      <Options>
        <OptionsHeading>{t('group_switches')}</OptionsHeading>
        <Option icon={<SwitchIcon />} end={elements.length} onClick={() => go('list')}>{t('switch_elements')}</Option>
        {!readOnly && <Option icon={<PlusIcon size={22} />} end={bare.length} onClick={() => go('bare')}>{t('switches_without')}</Option>}
      </Options>
      {!readOnly && <>
        <OptionsDivider />
        <Options>
          <OptionsHeading>{t('group_derailers')}</OptionsHeading>
          <Option icon={<DerailerIcon />} onClick={() => go('derailer')}>{t('create_derailer')}</Option>
        </Options>
      </>}
    </Panel>
  )
}

/** A switch element or derailer: how it is worked, its preferred position, and the flank protection it asks for. */
function ElementEditor({ element: chosen, domain, net, findings, ops, setMode, switchName, selected, readOnly, t, language }) {
  const markers = domain.collections.signals.filter(s => (s.frames ?? []).some(f => f.aspects.some(a => a.aspect === 'Ra_12')))
  const set = (patch) => ops.update('switchElements', { ...chosen, ...patch })
  return (
    <div className="tool-editor">
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
      <FlankSection element={chosen} domain={domain} findings={findings} ops={ops} readOnly={readOnly} t={t} selected={selected} />
      {chosen.derailer && (
        <Section title={t('location')}>
          <LocationFields loc={chosen.derailer.at} net={net} t={t} readOnly={readOnly}
            onChange={(at) => set({ derailer: { ...chosen.derailer, at } })}
            onPlace={() => setMode(placing('switchElements', (hit) => ({ ...chosen, derailer: { ...chosen.derailer, at: locationFrom(hit, { offset: chosen.derailer.at?.offset }) } }), (e) => ops.update('switchElements', e)))} />
        </Section>
      )}
    </div>
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
