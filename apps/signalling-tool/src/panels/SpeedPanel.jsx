import { useState } from 'react'
import { Badge, Button, NumberField, Panel, PlusIcon, Section, SelectField, SelectList, TextField } from '@rail-across/ui'
import { labelOf, newId } from '../model/domain.js'
import { runBetween } from '../model/runs.js'
import { locationFrom, placing, trackName } from '../model/places.js'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'

/**
 * The speed sections — the speed profile, as OSRD's SpeedSection has it: a
 * stretch from a start to an end over the run between them, the speed for all
 * trains and per tag of train where it differs, the direction it holds in,
 * the routes it holds on. A new one is set by two clicks on the plan, its
 * start and its end; the run between them is found over the net.
 */
export default function SpeedPanel({ domain, net, findings, selection, select, ops, setMode, readOnly, t }) {
  const { speedSections: list, routes, signals } = domain.collections
  const chosen = selection?.collection === 'speedSections' ? list.find(v => v.id === selection.id) : null
  const bad = new Set(findings.errors.filter(f => f.collection === 'speedSections').map(f => f.id))
  const where = (loc) => (loc?.anchor === 'station' ? `${trackName(net, loc.trackId)} · ${Math.round(loc.station)} m` : t('orphan'))
  const items = list.map(v => ({
    id: v.id,
    title: `${v.speed != null ? `${v.speed} km/h` : t('none')}${v.name ? ` · ${v.name}` : ''}`,
    sub: `${where(v.start)} → ${where(v.end)}`,
    end: bad.has(v.id) ? <Badge tone="danger">!</Badge> : null,
  })).sort((a, b) => a.sub.localeCompare(b.sub, 'de', { numeric: true }))
  const signalName = (id) => labelOf('signals', signals.find(s => s.id === id), t)
  const routeOptions = routes.map(r => ({ value: r.id, label: r.name || `${signalName(r.startSignalId)} → ${signalName(r.targetSignalId)}` }))

  /** Two clicks: the start, then the end; `make` the section they give, `apply` what to do with it. */
  const setTwice = (base, apply) => setMode(placing('speedSections',
    (hit) => ({ ...base, start: locationFrom(hit, { directed: false }), end: locationFrom(hit, { directed: false }), path: { trackIds: [hit.trackId] } }),
    (first) => setMode(placing('speedSections',
      (hit) => {
        // Only where a run leads from the start: elsewhere there is no section to set.
        const end = locationFrom(hit, { directed: false })
        const trackIds = runBetween(net, first.start, end)
        return trackIds ? { ...first, end, path: { trackIds } } : null
      },
      apply))))

  return (
    <Panel title={t('section_speeds')} intro={t('section_speeds_intro')}
      actions={!readOnly && (
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => setTwice({ id: newId(), speed: 60 }, (v) => ops.add('speedSections', v))}>
          {t('new_speed')}
        </Button>
      )}>
      <SelectList maxHeight="34vh" label={t('section_speeds')} items={items} selected={chosen?.id} onSelect={(id) => select({ collection: 'speedSections', id })} empty={t('empty_list')} />
      {chosen && (
        <SpeedEditor key={chosen.id} section={chosen} routeOptions={routeOptions} findings={findings} where={where} readOnly={readOnly} t={t}
          onChange={(v) => ops.update('speedSections', v)} onDelete={() => ops.remove('speedSections', chosen.id)}
          onPlace={() => setTwice(chosen, (v) => ops.update('speedSections', v))} />
      )}
    </Panel>
  )
}

function SpeedEditor({ section, routeOptions, findings, where, onChange, onDelete, onPlace, readOnly, t }) {
  const set = (patch) => onChange({ ...section, ...patch })
  const [tag, setTag] = useState('')
  const tags = Object.entries(section.speedByTag ?? {})
  const setTags = (entries) => set({ speedByTag: entries.length ? Object.fromEntries(entries) : undefined })
  return (
    <div className="tool-editor">
      <EditorHead title={section.speed != null ? `${section.speed} km/h` : t('section_speeds')} onDelete={onDelete} readOnly={readOnly} t={t} />
      <ObjectFindings findings={findings} collection="speedSections" id={section.id} t={t} />
      <div className="ui-fields">
        <TextField compact label={t('name')} value={section.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <NumberField compact label={t('speed')} value={section.speed ?? null} min={0} step={5} disabled={readOnly} onChange={(v) => set({ speed: v ?? undefined })} />
        <SelectField compact wide label={t('holds_in')} value={section.start?.direction ?? null} empty={t('both_ways')} disabled={readOnly}
          options={[{ value: 'up', label: t('up') }, { value: 'down', label: t('down') }]}
          onChange={(v) => set({ start: { ...section.start, direction: v ?? undefined } })} />
      </div>

      <Section title={t('speed_by_tag')}>
        {tags.map(([name, speed]) => (
          <div key={name} className="ui-row">
            <span className="ui-grow">{name}</span>
            <NumberField compact label={t('speed')} value={speed} min={0} step={5} disabled={readOnly}
              onChange={(v) => setTags(tags.map(([n, s]) => [n, n === name ? v ?? 0 : s]))} />
            {!readOnly && <button type="button" className="ui-link" onClick={() => setTags(tags.filter(([n]) => n !== name))}>{t('remove')}</button>}
          </div>
        ))}
        {!readOnly && (
          <div className="ui-row">
            <TextField compact label={t('tag')} value={tag} onChange={setTag} />
            <Button size="small" icon={<PlusIcon size={14} />} disabled={!tag.trim() || tags.some(([n]) => n === tag.trim())}
              onClick={() => { setTags([...tags, [tag.trim(), section.speed ?? 0]]); setTag('') }}>{t('add_tag')}</Button>
          </div>
        )}
      </Section>

      {routeOptions.length > 0 && (
        <Section title={t('on_routes')}>
          <p className="ui-muted">{t('on_routes_hint')}</p>
          <CheckList options={routeOptions} value={section.onRoutes} readOnly={readOnly} onChange={(ids) => set({ onRoutes: ids.length ? ids : undefined })} />
        </Section>
      )}

      <Section title={t('location')}>
        <p className="ui-muted">{where(section.start)} → {where(section.end)} · {t('path_tracks', { n: section.path?.trackIds?.length ?? 0 })}</p>
        {!readOnly && <div className="ui-row"><Button size="small" onClick={onPlace}>{t('place_speed')}</Button></div>}
      </Section>
    </div>
  )
}
