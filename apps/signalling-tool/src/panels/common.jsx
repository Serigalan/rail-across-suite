import { Button, Checkbox, NumberField, SelectField, TrashIcon } from '@rail-across/ui'
import { planePoint, trackName } from '../model/places.js'
import { findingText } from '../labels.js'
import { useTexts } from '../i18n.js'

/**
 * What every editor of an object of the domain has: the fields of its
 * location, a button to place it on the plan anew, one to delete it.
 */

/**
 * A location's fields: track, station, side, offset, direction (where
 * `directed`), and the point in the plane they give — where the plan draws
 * it; an orphan says so, and offers to be put on a track again.
 */
export function LocationFields({ loc, onChange, net, t, directed = true, readOnly, onPlace, withSide = true }) {
  if (!loc) return null
  if (loc.anchor === 'plane') {
    return (
      <div className="ui-section">
        <p className="ui-message ui-message-warn">{t('orphan')}</p>
        {!readOnly && onPlace && <Button size="small" onClick={onPlace}>{t('attach')}</Button>}
      </div>
    )
  }
  const set = (patch) => onChange({ ...loc, ...patch })
  const tracks = net.tracks.map(tr => ({ value: tr.id, label: trackName(net, tr.id) }))
  return (
    <div className="ui-fields">
      <SelectField compact wide label={t('track')} value={loc.trackId} options={tracks} disabled={readOnly} onChange={(v) => set({ trackId: v })} />
      <NumberField compact label={t('station')} value={loc.station} disabled={readOnly} onChange={(v) => v != null && set({ station: v })} />
      {withSide && (
        <SelectField compact label={t('side')} value={loc.side ?? null} empty={t('none')} disabled={readOnly}
          options={[{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]} onChange={(v) => set({ side: v ?? undefined })} />
      )}
      {withSide && <NumberField compact label={t('offset')} value={loc.offset ?? null} min={0} disabled={readOnly} onChange={(v) => set({ offset: v ?? undefined })} />}
      {directed && (
        <SelectField compact label={t('direction')} value={loc.direction ?? 'up'} disabled={readOnly}
          options={[{ value: 'up', label: t('up') }, { value: 'down', label: t('down') }]} onChange={(v) => set({ direction: v })} />
      )}
      <PlaneLine point={planePoint(net, loc)} t={t} />
      {!readOnly && onPlace && <div className="ui-wide ui-row"><Button size="small" onClick={onPlace}>{t('place_on_map')}</Button></div>}
    </div>
  )
}

/** The point in the plane a location gives, to the centimetre: easting, northing and the system. */
function PlaneLine({ point, t }) {
  if (!point) return null
  const m = (v) => v.toFixed(2)
  return <p className="ui-wide ui-muted tool-plane">{t('plane_point', { e: m(point.easting), n: m(point.northing), epsg: point.epsg })}</p>
}

/** The head of an editor: its title, and the button that deletes the object. */
export function EditorHead({ title, onDelete, readOnly, t, children }) {
  return (
    <div className="tool-editor-head">
      <h3 className="tool-editor-title">{title}</h3>
      {children}
      {!readOnly && onDelete && <Button size="small" variant="danger" icon={<TrashIcon size={14} />} onClick={onDelete}>{t('delete')}</Button>}
    </div>
  )
}

/** Checkboxes for picking ids out of `options` [{ value, label }]. */
export function CheckList({ options, value, onChange, readOnly }) {
  const set = new Set(value ?? [])
  return (
    <div className="tool-checklist">
      {options.map(o => (
        <Checkbox key={o.value} label={o.label} checked={set.has(o.value)} disabled={readOnly}
          onChange={(on) => onChange(on ? [...set, o.value] : [...set].filter(v => v !== o.value))} />
      ))}
    </div>
  )
}

/** The findings of the check on one object, under its editor. */
export function ObjectFindings({ findings, collection, id, t }) {
  const { language } = useTexts()
  const mine = [...findings.errors.map(f => ({ ...f, tone: 'error' })), ...findings.warnings.map(f => ({ ...f, tone: 'warn' }))]
    .filter(f => f.collection === collection && f.id === id)
  if (!mine.length) return null
  return (
    <ul className="tool-findings">
      {mine.map(f => <li key={f.key} className={`tool-finding tool-finding-${f.tone}`}>{findingText(f, t, language)}</li>)}
    </ul>
  )
}
