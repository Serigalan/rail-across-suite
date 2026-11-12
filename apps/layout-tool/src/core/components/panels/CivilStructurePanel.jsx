import { useState } from 'react'
import { generateId, stationFromClick, trackLength } from '@rail-across/core/track'
import { wgs84ToUTM } from '@rail-across/core/geometry'
import { deleteStructure, saveStructure, updateStructure } from '../../storage'
import { useStructures, useTracks } from '../../hooks/useStore'
import { useI18n } from '../../locales/i18nContext'
import useMapPick, { useSelectedOnMap } from '../../map/useMapPick'
import useMapEvents from '../../map/useMapEvents'
import { DEFAULT_OFFSETS } from '../../map/structureLayers'
import CommitBar from '../form/CommitBar'
import FormSection from '../form/FormSection'
import InfoTip from '../form/InfoTip'

/**
 * What each kind of civil structure states (decision 0025): its kinds, whether
 * it stands on a side of the track at a distance from its axis, its height,
 * and for a bridge its spans, what it crosses and the clear height under it.
 */
const STRUCTURE_KINDS = {
  bridges: { kinds: ['beam', 'arch', 'frame', 'truss', 'other'], side: false, fields: ['spans', 'crosses', 'clearance'] },
  retainingWalls: { kinds: ['cantilever', 'gravity', 'sheetPile', 'gabion', 'other'], side: true, fields: ['offset', 'height'], height: 3 },
  noiseBarriers: { kinds: ['absorbing', 'reflecting', 'transparent'], side: true, fields: ['offset', 'height'], height: 2 },
}

const round = (s) => Math.round(s * 100) / 100

/** A new structure's form on a track: its stretch, kind and what the kind states. */
function emptyForm(kind, trackId) {
  const def = STRUCTURE_KINDS[kind]
  return {
    trackId, from: null, to: null, name: '', kind: def.kinds[0], side: 'right',
    offset: DEFAULT_OFFSETS[kind] ?? null, height: def.height ?? null, spans: 1, crosses: '', clearance: null,
  }
}

/** The form of a structure kept. */
const formOf = (obj) => ({
  trackId: obj.at?.trackId, from: obj.at?.startStation, to: obj.at?.endStation, name: obj.name ?? '', kind: obj.kind, side: obj.at?.side ?? 'right',
  offset: obj.offset ?? null, height: obj.height ?? null, spans: obj.spans ?? null, crosses: obj.crosses ?? '', clearance: obj.clearance ?? null,
})

/** The structure a form makes, its id given. */
function structureOf(kind, id, f, keep = {}) {
  const def = STRUCTURE_KINDS[kind]
  const at = { anchor: 'station', trackId: f.trackId, startStation: round(Math.min(f.from, f.to)), endStation: round(Math.max(f.from, f.to)), ...(def.side ? { side: f.side } : {}) }
  const out = { ...keep, id, at, kind: f.kind }
  delete out.name
  if (f.name.trim()) out.name = f.name.trim()
  for (const k of def.fields) {
    const v = f[k]
    if (k === 'crosses') { if (v?.trim()) out.crosses = v.trim(); else delete out.crosses; continue }
    if (v == null || v === '' || !Number.isFinite(Number(v))) { delete out[k]; continue }
    out[k] = k === 'spans' ? Math.max(1, Math.round(Number(v))) : Number(v)
  }
  return out
}

/**
 * The Civil-Tool's bridges, retaining walls or noise barriers (`kind`): the
 * list of those there are, and one made or edited — its track picked on the
 * map, its beginning and end clicked on it (or typed as stations), its kind,
 * its side and distance from the axis, its height, a bridge's spans, what it
 * crosses and the clear height under it.
 */
export default function CivilStructurePanel({ kind }) {
  const { t, fill } = useI18n()
  const list = useStructures(kind)
  const tracks = useTracks()
  const def = STRUCTURE_KINDS[kind]
  const [editing, setEditing] = useState(null)   // null; { id: null } new; { id } kept
  const [f, setF] = useState(null)
  const [picking, setPicking] = useState(null)   // 'from' | 'to' | null
  const set = (patch) => setF(prev => ({ ...prev, ...patch }))
  const track = f?.trackId ? tracks.find(tr => tr.id === f.trackId) : null
  const trackName = (id) => tracks.find(tr => tr.id === id)?.name ?? id?.slice(0, 8) ?? '—'

  // A new one begins with its track, picked on the map.
  useMapPick({
    active: editing?.id === null && !f?.trackId, noSwitchBranch: true, hover: 'element',
    onPick: ({ trackId }) => { setF(emptyForm(kind, trackId)); setPicking('from') },
  })
  useSelectedOnMap(f?.trackId ? { trackId: f.trackId } : null)
  // …then its beginning and its end, on that track.
  const pickingStations = !!picking && !!track
  useMapPick({
    active: pickingStations, track: track?.id, cursor: false,
    onPick: ({ elementIndex }, e) => {
      const station = stationFromClick(track, elementIndex, wgs84ToUTM([e.lngLat.lng, e.lngLat.lat], track.epsg))
      if (station == null) return
      set({ [picking]: round(station) })
      setPicking(picking === 'from' ? 'to' : null)
    },
  })
  useMapEvents(pickingStations, {}, { cursor: 'crosshair' })

  const reset = () => { setEditing(null); setF(null); setPicking(null) }
  const length = track ? trackLength(track) : 0
  const valid = !!track && Number.isFinite(f?.from) && Number.isFinite(f?.to) && Math.abs(f.to - f.from) > 0.5
    && Math.min(f.from, f.to) >= -0.01 && Math.max(f.from, f.to) <= length + 0.01

  const commit = () => {
    if (!valid) return
    if (editing.id) updateStructure(kind, structureOf(kind, editing.id, f, list.find(o => o.id === editing.id)))
    else saveStructure(kind, structureOf(kind, generateId(), f))
    reset()
  }

  if (!editing) {
    return (
      <>
        <h2>{t(`civil_${kind}`)}<InfoTip text={t(`civil_${kind}_hint`)} /></h2>
        {list.length ? (
          <div className="create-element-options">
            {list.map(o => (
              <button key={o.id} className="ui-btn ui-btn-option" onClick={() => { setEditing({ id: o.id }); setF(formOf(o)) }}>
                {o.name || t(`civil_kind_${o.kind}`)}
                {` — ${trackName(o.at?.trackId)}, ${Math.round(o.at?.startStation ?? 0)}–${Math.round(o.at?.endStation ?? 0)} m`}
              </button>
            ))}
          </div>
        ) : <p>{t(`civil_${kind}_none`)}</p>}
        <button className="ui-btn ui-btn-primary ui-btn-block mt-8" onClick={() => { setEditing({ id: null }); setF(null) }}>{t(`civil_${kind}_new`)}</button>
      </>
    )
  }

  if (!f) {
    return (
      <>
        <h2>{t(`civil_${kind}_new`)}</h2>
        <p className="selecting-hint">{t('civil_pick_track')}</p>
        <CommitBar onCommit={() => {}} onCancel={reset} disabled />
      </>
    )
  }

  const number = (key, label) => (
    <div className="form-field">
      <label>{label}</label>
      <input type="number" step="any" value={f[key] ?? ''} onChange={e => set({ [key]: e.target.value === '' ? null : Number(e.target.value) })} />
    </div>
  )
  return (
    <>
      <h2>{t(editing.id ? `civil_${kind}_edit` : `civil_${kind}_new`)}</h2>
      {picking && <p className="selecting-hint">{t(picking === 'from' ? 'civil_pick_from' : 'civil_pick_to')}</p>}
      <FormSection title={fill('civil_on_track', { track: trackName(f.trackId) })}>
        {number('from', t('civil_from'))}
        {number('to', t('civil_to'))}
        <button className="ui-btn ui-btn-block" onClick={() => { set({ from: null, to: null }); setPicking('from') }}>{t('civil_repick')}</button>
      </FormSection>
      <FormSection title={t('section_meta')}>
        <div className="form-field">
          <label>{t('civil_name')}</label>
          <input type="text" value={f.name} onChange={e => set({ name: e.target.value })} />
        </div>
        <div className="form-field">
          <label>{t('civil_kind')}</label>
          <select value={f.kind} onChange={e => set({ kind: e.target.value })}>
            {def.kinds.map(k => <option key={k} value={k}>{t(`civil_kind_${k}`)}</option>)}
          </select>
        </div>
        {def.side && (
          <div className="form-field">
            <label>{t('civil_side')}</label>
            <select value={f.side} onChange={e => set({ side: e.target.value })}>
              <option value="left">{t('civil_side_left')}</option>
              <option value="right">{t('civil_side_right')}</option>
            </select>
          </div>
        )}
        {def.fields.includes('offset') && number('offset', t('civil_offset'))}
        {def.fields.includes('height') && number('height', t(`civil_height_${kind}`))}
        {def.fields.includes('spans') && number('spans', t('civil_spans'))}
        {def.fields.includes('crosses') && (
          <div className="form-field">
            <label>{t('civil_crosses')}</label>
            <input type="text" value={f.crosses} onChange={e => set({ crosses: e.target.value })} />
          </div>
        )}
        {def.fields.includes('clearance') && number('clearance', t('civil_clearance'))}
      </FormSection>
      {!valid && !picking && <p className="form-error">{t('civil_error_range')}</p>}
      {editing.id && (
        <button className="ui-btn ui-btn-danger ui-btn-block mt-2" onClick={() => { deleteStructure(kind, editing.id); reset() }}>{t(`civil_${kind}_delete`)}</button>
      )}
      <CommitBar onCommit={commit} onCancel={reset} disabled={!valid} />
    </>
  )
}
