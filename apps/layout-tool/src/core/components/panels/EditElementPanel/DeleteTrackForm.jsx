import { useState } from 'react'
import { deleteTrack, loadTracks, switchesOnTrack } from '../../../storage'
import { useI18n } from '../../../locales/i18nContext'
import CommitBar from '../../form/CommitBar'
import useMapPick, { useSelectedOnMap } from '../../../map/useMapPick'
import { useProject } from '../../../hooks/useStore'
import CancelButton from '../../form/CancelButton'
import GroupedTrackList from '../GroupedTrackList'
import { pickableTracks, trackTitle } from './trackPick'

export default function DeleteTrackForm({ onCommitted }) {
  const { t, fill } = useI18n()
  useProject()   // the list follows every write, the delete and its undo
  // { id, name, elements, switchNames } – switch branches are not offered: their
  // geometry belongs to a switch and goes with it, not on its own.
  const [selected, setSelected] = useState(null)
  const tracks = pickableTracks(loadTracks())

  const select = (trackId) => {
    const track = tracks.find(tr => tr.id === trackId)
    if (!track) return
    setSelected({
      id:          trackId,
      name:        trackTitle(track),
      elements:    (track.elements ?? []).length,
      switchNames: switchesOnTrack(trackId).map((sw, i) => sw.name || `#${i + 1}`),
    })
  }

  // Click to pick the track — on the map or in the list; the whole track is
  // highlighted, since the whole track is what gets deleted.
  useMapPick({ noSwitchBranch: true, hover: 'element', onPick: ({ trackId }) => select(trackId) })
  useSelectedOnMap(selected ? { trackId: selected.id } : null)

  // No question first (R10.5): the step notice offers the way back.
  const handleDelete = () => {
    deleteTrack(selected.id)
    setSelected(null)
  }

  return (
    <>
      <p>{t('edit_track_pick_hint')}</p>
      <GroupedTrackList tracks={tracks}
        isActive={(tr) => tr.id === selected?.id}
        onPick={(tr) => select(tr.id)} />
      {selected && (
        <>
          <p>{fill('edit_track_delete_selected', { name: selected.name, elements: selected.elements })}</p>
          {selected.switchNames.length > 0 && (
            <p className="form-error">{fill('edit_track_delete_switches', { names: selected.switchNames.join(', ') })}</p>
          )}
          <CommitBar onCommit={handleDelete} onCancel={onCommitted} commitLabel={t('edit_track_delete')} danger className="" />
        </>
      )}
      {!selected && <CancelButton className="ui-btn ui-btn-block mt-2" onClick={onCommitted} />}
    </>
  )
}
