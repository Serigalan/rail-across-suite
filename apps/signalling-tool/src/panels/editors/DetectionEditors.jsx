import { useMemo } from 'react'
import { Button, Checkbox, Section, SelectField, TextField } from '@rail-across/ui'
import { labelOf } from '../../model/domain.js'
import { POINT_KIND, SECTION_KIND, optionsOf } from '../../labels.js'
import { CheckList, EditorHead, LocationFields, ObjectFindings } from '../common.jsx'
import { locationFrom, pickingPoints, placing } from '../../model/places.js'

/**
 * A track section of train detection: bounded by its points, which are
 * picked from the list or clicked on the plan; its extent over the net is
 * derived and drawn while it is chosen.
 */
export function SectionEditor({ section, domain, findings, extents, ops, mode, setMode, readOnly, t, language }) {
  const points = domain.collections.detectionPoints
  const pointName = useMemo(() => new Map(points.map(p => [p.id, labelOf('detectionPoints', p, t)])), [points, t])
  const set = (patch) => ops.update('trackSections', { ...section, ...patch })
  const picking = mode?.kind === 'pick-points'
  return (
    <div className="tool-editor">
      <EditorHead title={labelOf('trackSections', section, t)} readOnly={readOnly} t={t} onDelete={() => ops.remove('trackSections', section.id)} />
      <ObjectFindings findings={findings} collection="trackSections" id={section.id} t={t} />
      <div className="ui-fields">
        <TextField compact label={t('name')} value={section.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <TextField compact label={t('area')} value={section.areaNumber ?? ''} disabled={readOnly} onChange={(v) => set({ areaNumber: v || undefined })} />
        <SelectField compact wide label={t('kind')} value={section.kind} options={optionsOf(SECTION_KIND, language)} disabled={readOnly}
          onChange={(v) => set({ kind: v })} />
        <div className="ui-wide"><Checkbox label={t('auxiliary_clearing')} checked={section.auxiliaryClearing} disabled={readOnly}
          onChange={(v) => set({ auxiliaryClearing: v || undefined })} /></div>
      </div>
      <Section title={t('extent')}>
        <ExtentText extent={extents.get(section.id)} t={t} />
      </Section>
      <Section title={t('bounded_by')} actions={!readOnly && (
        <Button size="small" active={picking} onClick={() => setMode(picking ? null : pickingPoints(ops, section.id))}>{t('pick_points')}</Button>
      )}>
        <CheckList readOnly={readOnly} value={section.boundedBy}
          options={[...points].sort((a, b) => String(pointName.get(a.id)).localeCompare(String(pointName.get(b.id)), 'de', { numeric: true }))
            .map(p => ({ value: p.id, label: pointName.get(p.id) }))}
          onChange={(boundedBy) => set({ boundedBy })} />
      </Section>
    </div>
  )
}

/** A detection point — an axle counter, a joint: its kind, where it lies, the sections it bounds. */
export function PointEditor({ point, domain, net, findings, ops, setMode, readOnly, t, language }) {
  const set = (patch) => ops.update('detectionPoints', { ...point, ...patch })
  const bounds = domain.collections.trackSections.filter(s => s.boundedBy.includes(point.id))
  return (
    <div className="tool-editor">
      <EditorHead title={labelOf('detectionPoints', point, t)} readOnly={readOnly} t={t} onDelete={() => ops.remove('detectionPoints', point.id)} />
      <ObjectFindings findings={findings} collection="detectionPoints" id={point.id} t={t} />
      <div className="ui-fields">
        <TextField compact label={t('name')} value={point.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <SelectField compact label={t('kind')} value={point.kind} options={optionsOf(POINT_KIND, language)} disabled={readOnly}
          onChange={(v) => set({ kind: v })} />
      </div>
      <Section title={t('location')}>
        <LocationFields loc={point.at} net={net} t={t} directed={false} readOnly={readOnly}
          onChange={(at) => set({ at })}
          onPlace={() => setMode(placing('detectionPoints', (hit) => ({ ...point, at: locationFrom(hit, { directed: false, offset: point.at?.offset }) }), (p) => ops.update('detectionPoints', p)))} />
      </Section>
      <p className="ui-muted">{t('track_sections')}: {bounds.map(s => labelOf('trackSections', s, t)).join(', ') || t('none')}</p>
    </div>
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
