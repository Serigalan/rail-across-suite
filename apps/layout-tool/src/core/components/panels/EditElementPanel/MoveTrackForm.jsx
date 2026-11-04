import { useState } from 'react'
import { loadTracks, moveTrack, switchesOnTrack } from '../../../storage'
import { useI18n } from '../../../locales/i18nContext'
import { formatNum } from '../../../locales/i18n'
import useMapPick, { useSelectedOnMap } from '../../../map/useMapPick'
import { useProject } from '../../../hooks/useStore'
import GroupedTrackList from '../GroupedTrackList'
import CommitBar from '../../form/CommitBar'
import NumberInput from '../../form/NumberInput'
import { pickableTracks, trackTitle } from './trackPick'

/**
 * Shift a whole track in its plane by an easting and a northing (x/y) — picked
 * on the map or from the list. The form stays on the track after a move, so a
 * second shift needs no second pick; another track is picked the same way.
 */
export default function MoveTrackForm({ onCommitted }) {
  const { t, fill, language } = useI18n()
  useProject()   // the list and the names follow every write, an undo too
  const [trackId, setTrackId] = useState(null)
  const [dx, setDx] = useState('')
  const [dy, setDy] = useState('')
  const [done, setDone] = useState(null)
  // Counts the moves: the fields start afresh after each, typed text and all.
  const [round, setRound] = useState(0)

  const tracks = pickableTracks(loadTracks())
  const track = tracks.find(tr => tr.id === trackId) ?? null
  const pick = (id) => { setTrackId(id); setDone(null) }

  useMapPick({ noSwitchBranch: true, hover: 'track', onPick: ({ trackId: id }) => pick(id) })
  useSelectedOnMap(track ? { trackId: track.id } : null)

  const east = Number(dx || 0), north = Number(dy || 0)
  const valid = Number.isFinite(east) && Number.isFinite(north) && (east !== 0 || north !== 0)
  const switchNames = track ? switchesOnTrack(track.id).map((sw, i) => sw.name || `#${i + 1}`) : []

  const handleCommit = () => {
    if (!track || !valid) return
    if (!moveTrack(track.id, east, north)) return
    setDone({ name: trackTitle(track), dx: formatNum(east, language), dy: formatNum(north, language) })
    setDx('')
    setDy('')
    setRound(n => n + 1)
  }

  return (
    <>
      <p>{t('edit_track_pick_hint')}</p>
      <GroupedTrackList tracks={tracks}
        isActive={(tr) => tr.id === trackId}
        onPick={(tr) => pick(tr.id)} />
      {track && (
        <div className="element-form">
          <p className="m-0"><strong>{fill('move_track_selected', { name: trackTitle(track) })}</strong></p>
          <p className="selecting-hint m-0">{t('move_track_hint')}</p>
          <div className="form-field">
            <label>{t('move_track_dx')}</label>
            <NumberInput key={`dx${round}`} value={dx} step="any" unit="m" onChange={(e) => { setDone(null); setDx(e.target.value) }} />
          </div>
          <div className="form-field">
            <label>{t('move_track_dy')}</label>
            <NumberInput key={`dy${round}`} value={dy} step="any" unit="m" onChange={(e) => { setDone(null); setDy(e.target.value) }} />
          </div>
          {switchNames.length > 0 && (
            <p className="form-error">{fill('move_track_switches', { names: switchNames.join(', ') })}</p>
          )}
        </div>
      )}
      {done && <p className="selecting-hint">{fill('move_track_done', done)}</p>}
      <CommitBar onCommit={handleCommit} onCancel={onCommitted} disabled={!track || !valid}
        commitLabel={t('move_track_commit')} cancelLabel={t('btn_back')} />
    </>
  )
}
