import { useState } from 'react'
import { loadSwitches, loadTracks, saveSwitch } from '../../../storage'
import { STAGE_LINK_KIND, newSwitchFields, trackEndAt } from '@rail-across/core/track'
import { endsMeeting, proposedRoles, stageLinkNote } from './stageLink'
import { wgs84ToUTM } from '@rail-across/core/geometry'
import { useI18n } from '../../../locales/i18nContext'
import useMapPick from '../../../map/useMapPick'
import CommitBar from '../../form/CommitBar'

const ROLES = ['A', 'B1', 'B2']

/**
 * A construction stage connector (decision 0027): three track ends at one
 * place joined without a switch — the track at A to the one at B1 while that
 * is in service, and to the one at B2 from the day it is. A click on the map
 * near the place finds the ends meeting there; their tracks' days propose
 * which takes which role.
 */
export default function StageLinkForm({ onCommitted }) {
  const { t } = useI18n()
  const [ends, setEnds] = useState(null)
  const [roles, setRoles] = useState(null)
  const [status, setStatus] = useState(null)

  useMapPick({
    active: true, hover: 'track',
    onPick: ({ trackId }, e) => {
      const tracks = loadTracks()
      const track = tracks.find(tr => tr.id === trackId)
      if (!track) return
      const click = wgs84ToUTM([e.lngLat.lng, e.lngLat.lat], track.epsg)
      const nearest = ['BEGIN', 'END'].map(ep => trackEndAt(track, ep)).filter(Boolean)
        .sort((a, b) => Math.hypot(a.easting - click[0], a.northing - click[1]) - Math.hypot(b.easting - click[0], b.northing - click[1]))[0]
      if (!nearest) return
      const found = endsMeeting(tracks, loadSwitches(), nearest)
      setEnds(found)
      if (found.length === 3) { setRoles(proposedRoles(found)); setStatus(null) }
      else { setRoles(null); setStatus(found.length < 3 ? 'stage_link_too_few' : 'stage_link_too_many') }
    },
  })

  const name = (e) => `${e.track.name || e.track.id.slice(0, 8)} ${e.endpoint === 'BEGIN' ? t('stage_link_begin') : t('stage_link_end')}`
  const days = (e) => [e.track.inServiceFrom && `${t('table_in_service_from')} ${e.track.inServiceFrom}`, e.track.inServiceTo && `${t('table_in_service_to')} ${e.track.inServiceTo}`].filter(Boolean).join(', ')
  const swap = (role, end) => setRoles(r => {
    const other = ROLES.find(k => r[k] === end)
    return { ...r, [role]: end, [other]: r[role] }
  })
  const note = roles ? stageLinkNote(roles) : null

  const commit = () => {
    if (!roles) return
    const ports = Object.fromEntries(ROLES.map(k => [k, { trackId: roles[k].track.id, endpoint: roles[k].endpoint }]))
    saveSwitch({ ...newSwitchFields(STAGE_LINK_KIND), ports })
    onCommitted?.()
  }

  return (
    <>
      <p className="selecting-hint">{t('stage_link_pick')}</p>
      {status && <p className="form-error">{t(status)}</p>}
      {roles && (
        <div className="form-list">
          {ROLES.map(role => (
            <div key={role} className="form-field">
              <label>{t(`stage_link_role_${role}`)}</label>
              <select value={ends.indexOf(roles[role])} onChange={e => swap(role, ends[Number(e.target.value)])}>
                {ends.map((end, i) => <option key={i} value={i}>{name(end)}{days(end) ? ` · ${days(end)}` : ''}</option>)}
              </select>
            </div>
          ))}
          {note && <p className="selecting-hint">{t(note)}</p>}
        </div>
      )}
      <CommitBar onCommit={commit} onCancel={onCommitted} disabled={!roles} commitLabel={t('stage_link_create')} className="" />
    </>
  )
}
