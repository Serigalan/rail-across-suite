import { useState } from 'react'
import { currentProject, loadTracks, replaceAllTracks } from '../../../storage'
import { pointOf } from '@rail-across/core/document'
import { SIDE_NAMES, buildTypeFields, lifespanWithStatus, trackStatus, withLifespan, DEFAULT_HEIGHT_EPSG, trackTypeName } from '@rail-across/core/track'
import useTrackFields from '../../../hooks/useTrackFields'
import TrackFields from '../TrackFields'
import StatusField from '../StatusField'
import HeightDatumField from '../HeightDatumField'
import { useI18n } from '../../../locales/i18nContext'
import useMapPick, { useSelectedOnMap } from '../../../map/useMapPick'
import CommitBar from '../../form/CommitBar'
import FormSection from '../../form/FormSection'
import CancelButton from '../../form/CancelButton'
import { firstReason } from '../../form/firstReason'

export default function EditPropertiesForm({ onCommitted }) {
  const { t } = useI18n()
  const { fields, errors, setErrors, setField, lineNumberError } = useTrackFields()
  const [selectedTrackId, setSelectedTrackId] = useState(null)
  const [name, setName] = useState('')
  const [nameError, setNameError] = useState(false)
  const [status, setStatus] = useState('existing')


  // The properties belong to the track, so the whole track is highlighted —
  // clicking one of its elements only says which track is meant.
  useMapPick({
    active: selectedTrackId === null, noSwitchBranch: true, hover: 'element',
    onPick: ({ trackId }) => {
      const track = loadTracks().find(tr => tr.id === trackId)
      if (!track) return
      setSelectedTrackId(trackId)
      setName(track.name ?? '')
      setStatus(trackStatus(track))
      setField('owner',       track.owner       ?? 'DB')
      setField('type',        trackTypeName(track, 'line_track'))
      setField('lineNumber',  track.lineNumber  ?? '')
      setField('lineName',    track.lineName    ?? '')
      setField('side',        SIDE_NAMES[track.side]        ?? 'sorting')
      // The station is the point the track names (decision 0028); a name typed
      // is the point of that name, found or made, when the form is taken.
      const point = pointOf(currentProject(), track)
      setField('stationName', point?.name ?? '')
      setField('uicStation',  point?.uic  ?? '')
      setField('trackNumber', track.trackNumber ?? '')
      setField('trackUse',    track.trackUse    ?? 'main')
      setField('heightEpsg',  String(track.heightEpsg ?? DEFAULT_HEIGHT_EPSG))
    },
  })
  useSelectedOnMap(selectedTrackId ? { trackId: selectedTrackId } : null)

  const handleCommit = () => {
    setErrors([])
    if (lineNumberError) return

    const existingNames = new Set(
      loadTracks().filter(t => t.id !== selectedTrackId).map(t => t.name).filter(Boolean)
    )
    if (name && existingNames.has(name)) {
      setNameError(true)
      return
    }
    setNameError(false)

    const tracks = loadTracks()
    if (!tracks.some(t => t.id === selectedTrackId)) return

    // Only the selected track is written — the properties describe this track,
    // and a neighbour that happens to connect to it may well be a different line.
    // The status is the lifespan's (decision 0028): existing none, the days typed in the track table kept.
    const newTracks = tracks.map(track => track.id !== selectedTrackId ? track : withLifespan({
      ...track,
      name,
      owner: fields.owner,
      ...buildTypeFields(fields),
    }, lifespanWithStatus(track.lifespan, status)))

    replaceAllTracks(newTracks)
    onCommitted?.()
  }

  return (
    <>
      {!selectedTrackId ? (
        <p>{t('edit_element_hint')}</p>
      ) : (
        <>
          <FormSection title={t('section_meta')}>
            <TrackFields fields={fields} setField={setField} setErrors={setErrors} errors={errors}
              name={name} onNameChange={(val) => { setName(val); setNameError(false) }} nameError={nameError} />
            <StatusField value={status} onChange={setStatus} />
          </FormSection>
          <FormSection title={t('section_geometry')}>
            <HeightDatumField value={fields.heightEpsg} onChange={v => setField('heightEpsg', v)} />
          </FormSection>
        </>
      )}
      {selectedTrackId ? (
        <>
          <CommitBar onCommit={handleCommit} onCancel={onCommitted} reason={firstReason(lineNumberError && t(`line_number_error_${lineNumberError}`))} />
        </>
      ) : (
        <CancelButton className="ui-btn ui-btn-block mt-8" onClick={onCommitted} />
      )}
    </>
  )
}
