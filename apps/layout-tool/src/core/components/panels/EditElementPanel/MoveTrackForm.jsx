import { useState } from 'react'
import { loadTracks, moveTrack, switchesOnTrack } from '../../../storage'
import { useI18n } from '../../../locales/i18nContext'
import { formatNum } from '../../../locales/i18n'
import useMapPick, { useSelectedOnMap } from '../../../map/useMapPick'
import useMapEvents from '../../../map/useMapEvents'
import useDrawPreview from '../../../map/useDrawPreview'
import { useProject } from '../../../hooks/useStore'
import GroupedTrackList from '../GroupedTrackList'
import CommitBar from '../../form/CommitBar'
import NumberInput from '../../form/NumberInput'
import ReadOnlyField from '../../form/ReadOnlyField'
import { passPoint, pickableTracks, shiftBetween, trackTitle } from './trackPick'

/**
 * Shift a whole track in its plane (x/y) by two pass points: the track picked
 * on the map or from the list, then a point where it lies now and the point it
 * is to lie at — each click takes a track's node where it comes near one, so
 * an end goes exactly onto another. The shift between them fills the fields,
 * which stay open to be set by hand. The form stays on the track after a move,
 * so a second shift needs no second pick of it.
 */
export default function MoveTrackForm({ onCommitted }) {
  const { t, fill, language } = useI18n()
  useProject()   // the list and the names follow every write, an undo too
  const draw = useDrawPreview()
  const [trackId, setTrackId] = useState(null)
  const [from, setFrom] = useState(null)   // pass point 1: { easting, northing, wgs, snapped }
  const [to, setTo] = useState(null)       // pass point 2
  const [dx, setDx] = useState('')
  const [dy, setDy] = useState('')
  const [done, setDone] = useState(null)
  // Counts the times the fields are set from outside: they start afresh, typed text and all.
  const [round, setRound] = useState(0)

  const all = loadTracks()
  const tracks = pickableTracks(all)
  const track = tracks.find(tr => tr.id === trackId) ?? null

  const setShift = (x, y) => { setDx(x); setDy(y); setRound(n => n + 1) }
  const restart = () => { setFrom(null); setTo(null); setShift('', ''); draw.clear() }
  const pick = (id) => { setTrackId(id); setDone(null); restart() }

  // The track first; then the clicks are the pass points.
  useMapPick({ active: !track, noSwitchBranch: true, hover: 'track', onPick: ({ trackId: id }) => pick(id) })
  useSelectedOnMap(track ? { trackId: track.id } : null)

  const label = (shift) => `Δ ${formatNum(shift.dx, language, { digits: 3 })} / ${formatNum(shift.dy, language, { digits: 3 })} m`
  useMapEvents(Boolean(track) && !to, {
    click: (e, m) => {
      const point = passPoint(all, track.epsg, { lngLat: [e.lngLat.lng, e.lngLat.lat], point: e.point }, (wgs) => m.project(wgs))
      setDone(null)
      if (!from) {
        setFrom(point)
        draw.markers([point.wgs])
        return
      }
      setTo(point)
      const shift = shiftBetween(from, point)
      setShift(String(shift.dx), String(shift.dy))
      draw.line([from.wgs, point.wgs], label(shift))
      draw.markers([from.wgs, point.wgs])
    },
    mousemove: (e, m) => {
      if (!from) return
      const point = passPoint(all, track.epsg, { lngLat: [e.lngLat.lng, e.lngLat.lat], point: e.point }, (wgs) => m.project(wgs))
      draw.line([from.wgs, point.wgs], label(shiftBetween(from, point)))
      draw.markers(point.snapped ? [from.wgs, point.wgs] : [from.wgs])
    },
  }, { cursor: 'crosshair' })

  const east = Number(dx || 0), north = Number(dy || 0)
  const valid = Number.isFinite(east) && Number.isFinite(north) && (east !== 0 || north !== 0)
  const switchNames = track ? switchesOnTrack(track.id).map((sw, i) => sw.name || `#${i + 1}`) : []

  const handleCommit = () => {
    if (!track || !valid) return
    if (!moveTrack(track.id, east, north)) return
    setDone({ name: trackTitle(track), dx: formatNum(east, language), dy: formatNum(north, language) })
    restart()
  }

  const coords = (p) => fill('move_track_point', {
    e: formatNum(p.easting, language, { digits: 3 }), n: formatNum(p.northing, language, { digits: 3 }),
  })

  return (
    <>
      <p>{t('edit_track_pick_hint')}</p>
      <GroupedTrackList tracks={tracks}
        isActive={(tr) => tr.id === trackId}
        onPick={(tr) => pick(tr.id)} />
      {track && (
        <div className="element-form">
          <p className="m-0"><strong>{trackTitle(track)}</strong></p>
          <p className="selecting-hint m-0">{t(!from ? 'move_track_pick_from' : !to ? 'move_track_pick_to' : 'move_track_hint')}</p>
          <ReadOnlyField label={`${t('move_track_from')}${from?.snapped ? ` · ${t('move_track_node')}` : ''}`} value={from ? coords(from) : '–'} />
          <ReadOnlyField label={`${t('move_track_to')}${to?.snapped ? ` · ${t('move_track_node')}` : ''}`} value={to ? coords(to) : '–'} />
          {from && (
            <button type="button" className="ui-btn ui-btn-block" onClick={restart}>{t('move_track_repick')}</button>
          )}
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
