import { useMemo, useState } from 'react'
import { Badge, Button, Checkbox, Panel, PlusIcon, Section, SelectField, SelectList, Tabs, TextField } from '@rail-across/ui'
import { newId, labelOf } from '../model/domain.js'
import { POINT_KIND, SECTION_KIND, optionsOf, word } from '../labels.js'
import { CheckList, EditorHead, LocationFields, ObjectFindings } from './common.jsx'
import { locationFrom, trackName } from '../model/places.js'

/**
 * Train detection: the points that bound sections — axle counters, joints —
 * and the sections, each bounded by its points; its extent over the net is
 * derived and drawn when it is chosen. A section's points are picked from the
 * list or clicked on the plan.
 */
export default function DetectionPanel({ domain, net, findings, extents, selection, select, ops, setMode, mode, readOnly, t, language }) {
  const [tab, setTab] = useState(selection?.collection === 'detectionPoints' ? 'points' : 'sections')
  const { detectionPoints: points, trackSections: sections } = domain.collections
  const pointName = useMemo(() => new Map(points.map(p => [p.id, labelOf('detectionPoints', p, t)])), [points, t])
  const chosenPoint = selection?.collection === 'detectionPoints' ? points.find(p => p.id === selection.id) : null
  const chosenSection = selection?.collection === 'trackSections' ? sections.find(s => s.id === selection.id) : null
  const warned = new Set(findings.warnings.filter(f => f.collection === 'trackSections').map(f => f.id))

  const sectionItems = sections.map(s => {
    const ext = extents.get(s.id)
    const m = ext ? Math.round(ext.parts.reduce((sum, p) => sum + (p.to - p.from), 0)) : 0
    return {
      id: s.id, title: labelOf('trackSections', s, t),
      sub: `${word(SECTION_KIND, s.kind, language)} · ${s.boundedBy.length} × · ${m} m`,
      end: warned.has(s.id) ? <Badge tone="warn">!</Badge> : null,
    }
  }).sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))
  const pointItems = points.map(p => ({
    id: p.id, title: pointName.get(p.id),
    sub: [word(POINT_KIND, p.kind, language), p.at?.anchor === 'station' ? `${trackName(net, p.at.trackId)} · ${Math.round(p.at.station)} m` : t('orphan')].join(' · '),
  })).sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { numeric: true }))

  const pickPoints = (section) => setMode({
    kind: 'pick-points',
    toggle: (id) => ops.modify('trackSections', section.id, (now) => ({
      ...now, boundedBy: now.boundedBy.includes(id) ? now.boundedBy.filter(x => x !== id) : [...now.boundedBy, id],
    })),
  })

  return (
    <Panel title={t('section_detection')} intro={t('section_detection_intro')}
      actions={!readOnly && <>
        <Button size="small" variant="primary" icon={<PlusIcon size={14} />} disabled={!net.tracks.length}
          onClick={() => setMode({ kind: 'place', apply: (hit) => ops.add('detectionPoints', { id: newId(), kind: 'axleCounter', at: locationFrom(hit, { direction: null }) }) })}>
          {t('new_point')}
        </Button>
        <Button size="small" icon={<PlusIcon size={14} />} disabled={!points.length}
          onClick={() => { const s = { id: newId(), kind: 'axleCounting', boundedBy: [] }; ops.add('trackSections', s); setTab('sections'); pickPoints(s) }}>
          {t('new_section')}
        </Button>
      </>}>
      <Tabs label={t('section_detection')} value={tab} onChange={setTab}
        options={[{ value: 'sections', label: `${t('track_sections')} (${sections.length})` }, { value: 'points', label: `${t('detection_points')} (${points.length})` }]} />
      {tab === 'sections'
        ? <SelectList maxHeight="34vh" label={t('track_sections')} items={sectionItems} selected={chosenSection?.id} onSelect={(id) => select({ collection: 'trackSections', id })} empty={t('empty_list')} />
        : <SelectList maxHeight="34vh" label={t('detection_points')} items={pointItems} selected={chosenPoint?.id} onSelect={(id) => select({ collection: 'detectionPoints', id })} empty={t('empty_list')} />}

      {chosenSection && (
        <div className="tool-editor" key={chosenSection.id}>
          <EditorHead title={labelOf('trackSections', chosenSection, t)} readOnly={readOnly} t={t} onDelete={() => ops.remove('trackSections', chosenSection.id)} />
          <ObjectFindings findings={findings} collection="trackSections" id={chosenSection.id} t={t} />
          <div className="ui-fields">
            <TextField compact label={t('name')} value={chosenSection.name ?? ''} disabled={readOnly} onChange={(v) => ops.update('trackSections', { ...chosenSection, name: v || undefined })} />
            <TextField compact label={t('area')} value={chosenSection.areaNumber ?? ''} disabled={readOnly} onChange={(v) => ops.update('trackSections', { ...chosenSection, areaNumber: v || undefined })} />
            <SelectField compact wide label={t('kind')} value={chosenSection.kind} options={optionsOf(SECTION_KIND, language)} disabled={readOnly}
              onChange={(v) => ops.update('trackSections', { ...chosenSection, kind: v })} />
            <div className="ui-wide"><Checkbox label={t('auxiliary_clearing')} checked={chosenSection.auxiliaryClearing} disabled={readOnly}
              onChange={(v) => ops.update('trackSections', { ...chosenSection, auxiliaryClearing: v || undefined })} /></div>
          </div>
          <Section title={t('extent')}>
            <ExtentText extent={extents.get(chosenSection.id)} t={t} />
          </Section>
          <Section title={t('bounded_by')} actions={!readOnly && (
            <Button size="small" active={mode?.kind === 'pick-points'} onClick={() => (mode?.kind === 'pick-points' ? setMode(null) : pickPoints(chosenSection))}>{t('pick_points')}</Button>
          )}>
            <CheckList readOnly={readOnly} value={chosenSection.boundedBy}
              options={[...points].sort((a, b) => String(pointName.get(a.id)).localeCompare(String(pointName.get(b.id)), 'de', { numeric: true }))
                .map(p => ({ value: p.id, label: pointName.get(p.id) }))}
              onChange={(boundedBy) => ops.update('trackSections', { ...chosenSection, boundedBy })} />
          </Section>
        </div>
      )}

      {chosenPoint && (
        <div className="tool-editor" key={chosenPoint.id}>
          <EditorHead title={pointName.get(chosenPoint.id)} readOnly={readOnly} t={t} onDelete={() => ops.remove('detectionPoints', chosenPoint.id)} />
          <ObjectFindings findings={findings} collection="detectionPoints" id={chosenPoint.id} t={t} />
          <div className="ui-fields">
            <TextField compact label={t('name')} value={chosenPoint.name ?? ''} disabled={readOnly} onChange={(v) => ops.update('detectionPoints', { ...chosenPoint, name: v || undefined })} />
            <SelectField compact label={t('kind')} value={chosenPoint.kind} options={optionsOf(POINT_KIND, language)} disabled={readOnly}
              onChange={(v) => ops.update('detectionPoints', { ...chosenPoint, kind: v })} />
          </div>
          <Section title={t('location')}>
            <LocationFields loc={chosenPoint.at} net={net} t={t} directed={false} readOnly={readOnly}
              onChange={(at) => ops.update('detectionPoints', { ...chosenPoint, at })}
              onPlace={() => setMode({ kind: 'place', apply: (hit) => ops.update('detectionPoints', { ...chosenPoint, at: locationFrom(hit, { direction: null }) }) })} />
          </Section>
          <p className="ui-muted">{t('track_sections')}: {sections.filter(s => s.boundedBy.includes(chosenPoint.id)).map(s => labelOf('trackSections', s, t)).join(', ') || t('none')}</p>
        </div>
      )}
    </Panel>
  )
}

function ExtentText({ extent, t }) {
  if (!extent) return null
  const m = Math.round(extent.parts.reduce((sum, p) => sum + (p.to - p.from), 0))
  return <>
    <p className="ui-muted">{t('extent_m', { m, n: new Set(extent.parts.map(p => p.trackId)).size })}</p>
    {!extent.matches && <p className="ui-message ui-message-warn">{t('extent_differs')}</p>}
  </>
}
