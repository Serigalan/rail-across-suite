import { useEffect, useMemo, useState } from 'react'
import { loadTracks, loadSwitches, loadEndMarks, saveEndMark, deleteEndMark } from '../../../storage'
import {
  classifyTrackEnds, freeEnds, BUFFER_STOP_TYPES, DEFAULT_BUFFER_STOP_TYPE, BUFFER_STOP_LENGTH,
  BUFFER_STOP, BOUNDARY, defaultBrakeLength, bufferStopFits, newBufferStop, newBoundary, endMarkName,
  bufferStopFeatures, boundaryFeatures, trackLength,
} from '@rail-across/core/track'
import { useI18n } from '../../../locales/i18nContext'
import CommitBar from '../../form/CommitBar'
import { useProject } from '../../../hooks/useStore'
import usePreview from '../../../map/usePreview'
import useMapEvents from '../../../map/useMapEvents'
import { PALETTE } from '../../../styles/palette'
import CancelButton from '../../form/CancelButton'
import NumberInput from '../../form/NumberInput'
import { splitUnit } from '../../../locales/i18n'
import InfoTip from '../../form/InfoTip'

/** How close (px) the cursor has to come to an end for it to be the one meant. */
const PICK_PX = 14

const ENDS_SOURCE   = 'buffer-stop-ends-source'
const SHAPE_SOURCE  = 'buffer-stop-preview-source'
const BRAKE_SOURCE  = 'buffer-stop-preview-brake-source'

const PREVIEW_LAYERS = [
  {
    sourceId: BRAKE_SOURCE,
    layer: {
      id: 'buffer-stop-preview-brake-layer', type: 'line',
      paint: { 'line-color': PALETTE.mapSelected, 'line-width': 3, 'line-dasharray': [1.5, 1] },
    },
  },
  {
    sourceId: SHAPE_SOURCE,
    layer: {
      id: 'buffer-stop-preview-layer', type: 'line',
      paint: { 'line-color': PALETTE.mapSelected, 'line-width': 5 },
    },
  },
  {
    sourceId: ENDS_SOURCE,
    layer: {
      id: 'buffer-stop-ends-layer', type: 'circle',
      paint: {
        'circle-radius': ['case', ['get', 'active'], 7, 5],
        'circle-color': ['case', ['get', 'active'], PALETTE.mapHover, PALETTE.mapCandidate],
        'circle-stroke-color': PALETTE.white,
        'circle-stroke-width': 1.5,
      },
    },
  },
]

const keyOf = (end) => (end ? `${end.trackId}|${end.endpoint}` : null)

/** A number typed in, or NaN; an empty field is `empty`. */
const numberOf = (text, empty = NaN) => (text.trim() === '' ? empty : Number(text.replace(',', '.')))

/**
 * Put a buffer stop or — `kind` BOUNDARY — a boundary of the planning area on
 * a free track end, or — `edit` — change or remove one that stands. The ends
 * to choose from are drawn on the map as dots, and the one under the cursor
 * is the one a click takes: an end is a point, and a point is easier hit from
 * a dot than from a line that runs on either side of it.
 *
 * Type and brake length are the whole of a buffer stop (ROADMAP decisions 76,
 * 77): the type proposes the brake length until the brake length has been
 * typed in by hand, and the stop is 2.20 m long in front of it. A boundary is
 * where the line runs on undrawn; a train crosses it at its speed in a
 * simulation, or at what the line allows there. A train may begin and end at
 * either, under the mark's name — the one proposed until another is typed.
 */
export default function EndMarkForm({ onCommitted, edit = false, kind = BUFFER_STOP }) {
  const { t, fill } = useI18n()
  const project = useProject()
  const boundary = kind === BOUNDARY
  const [version, setVersion] = useState(0)
  const { tracks, candidates, marks } = useMemo(() => {
    const tracks = loadTracks()
    const marks = loadEndMarks()
    const ends = classifyTrackEnds(tracks, loadSwitches(), marks)
    const candidates = edit ? ends.filter(e => e.state === kind) : freeEnds(ends)
    return { tracks, candidates, marks }
    // `version` re-reads the store after a commit in place.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.id, edit, kind, version])

  const [hover, setHover]       = useState(null)
  const [picked, setPicked]     = useState(null)
  const [name, setName]         = useState('')
  const [type, setType]         = useState(DEFAULT_BUFFER_STOP_TYPE)
  const [brake, setBrake]       = useState(String(defaultBrakeLength(DEFAULT_BUFFER_STOP_TYPE)))
  const [brakeTyped, setBrakeTyped] = useState(false)
  const [speed, setSpeed]       = useState('')

  const preview = usePreview(PREVIEW_LAYERS, { resetCursor: true })

  const track = picked ? tracks.find(tr => tr.id === picked.trackId) : null
  const mark  = picked && edit ? marks.find(m => m.id === picked.markId) : null
  const have  = track ? trackLength(track) : 0
  const brakeValue = numberOf(brake)
  const brakeInvalid = !boundary && !(Number.isFinite(brakeValue) && brakeValue >= 0)
  const tooShort = !boundary && !brakeInvalid && track && !bufferStopFits(have, brakeValue)
  const speedValue = numberOf(speed, null)
  const speedInvalid = boundary && speedValue != null && !(Number.isFinite(speedValue) && speedValue > 0)
  const invalid = brakeInvalid || tooShort || speedInvalid

  // The dots, the one under the cursor or picked drawn larger.
  useEffect(() => {
    const active = new Set([keyOf(hover), keyOf(picked)].filter(Boolean))
    preview.set(ENDS_SOURCE, {
      type: 'FeatureCollection',
      features: candidates.map(e => ({
        type: 'Feature',
        properties: { active: active.has(keyOf(e)) },
        geometry: { type: 'Point', coordinates: e.lngLat },
      })),
    })
  }, [preview, candidates, hover, picked])

  // The mark as it would stand.
  useEffect(() => {
    let feats = []
    if (track && boundary) feats = boundaryFeatures([track], [newBoundary(track.id, picked.endpoint)])
    else if (track && !brakeInvalid) feats = bufferStopFeatures([track], [newBufferStop(track.id, picked.endpoint, type, brakeValue)])
    preview.set(SHAPE_SOURCE, { type: 'FeatureCollection', features: feats.filter(f => f.properties.part !== 'brake') })
    preview.set(BRAKE_SOURCE, { type: 'FeatureCollection', features: feats.filter(f => f.properties.part === 'brake') })
  }, [track, picked, boundary, type, brakeValue, brakeInvalid, preview])

  // Picking: the nearest end within PICK_PX of the cursor.
  const nearest = (m, point) => {
    let best = null, bestD = PICK_PX
    for (const end of candidates) {
      const p = m.project(end.lngLat)
      const d = Math.hypot(p.x - point.x, p.y - point.y)
      if (d < bestD) { best = end; bestD = d }
    }
    return best
  }
  useMapEvents(true, {
    mousemove: (e, m) => {
      const end = nearest(m, e.point)
      m.getCanvas().style.cursor = end ? 'pointer' : ''
      setHover(prev => (keyOf(prev) === keyOf(end) ? prev : end))
    },
    click: (e, m) => {
      const end = nearest(m, e.point)
      if (!end) return
      setPicked(end)
      const existing = edit ? loadEndMarks().find(mk => mk.id === end.markId) : null
      setName(existing?.name ?? '')
      setSpeed(existing?.speed != null ? String(existing.speed) : '')
      if (existing && !boundary) {
        setType(existing.type ?? DEFAULT_BUFFER_STOP_TYPE)
        setBrake(String(existing.brakeLength ?? 0))
        setBrakeTyped(true)
      }
    },
  })

  const clearMap = () => preview.clear()

  const handleType = (value) => {
    const next = Number(value)
    setType(next)
    // The type proposes the brake length only until somebody has said otherwise.
    if (!brakeTyped) setBrake(String(defaultBrakeLength(next)))
  }

  const finish = () => {
    setPicked(null)
    setHover(null)
    setVersion(v => v + 1)
    onCommitted?.()
  }

  // A name left empty is the one proposed: it is not stored, and follows the track's.
  const named = (record) => {
    const { name: _n, speed: _s, ...rest } = record
    return {
      ...rest,
      ...(name.trim() ? { name: name.trim() } : {}),
      ...(boundary && speedValue != null ? { speed: speedValue } : {}),
    }
  }

  const handleCommit = () => {
    if (!track || invalid) return
    let record
    if (boundary) record = mark ?? newBoundary(track.id, picked.endpoint)
    else record = mark ? { ...mark, type, brakeLength: brakeValue } : newBufferStop(track.id, picked.endpoint, type, brakeValue)
    saveEndMark(named(record))
    clearMap()
    finish()
  }

  const handleDelete = () => {
    if (!mark) return
    deleteEndMark(mark.id)
    clearMap()
    finish()
  }

  const endName = (end) => fill('buffer_stop_at', {
    track: tracks.find(tr => tr.id === end.trackId)?.name || end.trackId.slice(0, 8),
    end: t(end.endpoint === 'BEGIN' ? 'end_begin' : 'end_end'),
  })
  const words = boundary
    ? { none: 'boundary_none', noneExisting: 'boundary_none_existing', pick: 'buffer_stop_pick', pickExisting: 'boundary_pick_existing', remove: 'boundary_delete' }
    : { none: 'buffer_stop_none', noneExisting: 'buffer_stop_none_existing', pick: 'buffer_stop_pick', pickExisting: 'buffer_stop_pick_existing', remove: 'buffer_stop_delete' }

  if (!candidates.length) {
    return (
      <>
        <p className="selecting-hint">{t(edit ? words.noneExisting : words.none)}</p>
        <CancelButton className="ui-btn ui-btn-block mt-2" onClick={onCommitted} />
      </>
    )
  }

  return (
    <>
      {!picked && <p className="selecting-hint">{t(edit ? words.pickExisting : words.pick)}</p>}

      {picked && (
        <div className="element-form">
          <p className="m-0"><strong>{endName(picked)}</strong></p>
          <div className="form-field">
            <label>{t('end_mark_name')}<InfoTip text={t('end_mark_name_hint')} /></label>
            <input type="text" value={name} onChange={e => setName(e.target.value)}
              placeholder={endMarkName({ kind, trackId: picked.trackId, endpoint: picked.endpoint }, track)} />
          </div>
          {boundary ? (
            <div className="form-field">
              <label>{splitUnit(t('boundary_speed')).text}<InfoTip text={t('boundary_speed_hint')} /></label>
              <NumberInput min="0" step="5" value={speed} onChange={e => setSpeed(e.target.value)} unit="km/h"
                placeholder={t('boundary_speed_line')} />
            </div>
          ) : (
            <>
              <div className="form-field">
                <label>{t('buffer_stop_type')}</label>
                <select value={type} onChange={e => handleType(e.target.value)}>
                  {BUFFER_STOP_TYPES.map(v => <option key={v} value={v}>{v}</option>)}
                </select>
              </div>
              <div className="form-field">
                <label>{splitUnit(t('buffer_stop_brake')).text}<InfoTip text={t('buffer_stop_hint')} /></label>
                <NumberInput min="0" step="0.1" value={brake}
                  onChange={e => { setBrake(e.target.value); setBrakeTyped(true) }} unit="m" />
              </div>
            </>
          )}
        </div>
      )}

      {picked && brakeInvalid && <p className="form-error">{t('buffer_stop_brake_invalid')}</p>}
      {picked && tooShort && (
        <p className="form-error">{fill('buffer_stop_too_short', {
          need: (brakeValue + BUFFER_STOP_LENGTH).toFixed(2), have: have.toFixed(2),
        })}</p>
      )}
      {picked && speedInvalid && <p className="form-error">{t('boundary_speed_invalid')}</p>}

      {picked && mark && (
        <button className="ui-btn ui-btn-danger ui-btn-block" onClick={handleDelete}>
          {t(words.remove)}
        </button>
      )}
      {picked
        ? <CommitBar onCommit={handleCommit} onCancel={onCommitted} disabled={invalid} className={mark ? 'mt-2' : ''} />
        : <CancelButton className="ui-btn ui-btn-block mt-2" onClick={onCommitted} />}
    </>
  )
}
