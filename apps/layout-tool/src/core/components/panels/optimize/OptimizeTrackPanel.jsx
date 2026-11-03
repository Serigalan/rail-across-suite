import { useEffect, useRef, useState } from 'react'
import { loadTracks, updateTrack } from '../../../storage'
import { trackLabel } from '@rail-across/core/track'
import { KATALOG } from '@rail-across/core/rules'
import { runOptimizer } from '../../../utils/optimizer'
import { moduleErrorText } from './moduleErrorText'
import { reconstructElements } from '@rail-across/core/geometry'
import { ZOOM_LINE_WIDTH } from '../../../map/style'
import { optimizeRequest, optimizedTrack, optimizeErrorText } from '@rail-across/core/commands'
import { OptimizeTrackModeIcon, OptimizeElementModeIcon, AxisFitModeIcon } from '../../icons'
import { useI18n } from '../../../locales/i18nContext'
import { useMap } from '../../../map/MapContext'
import { TRACKS_HOVER_LAYER } from '../../../map/layerIds'
import usePreview from '../../../map/usePreview'
import useMapPick from '../../../map/useMapPick'
import { PALETTE } from '../../../styles/palette'
import CommitBar from '../../form/CommitBar'
import useElapsed from '../../../hooks/useElapsed'
import OptimizeSettings from './OptimizeSettings'
import OptimizeResult from './OptimizeResult'
import AxisFitPanel from './AxisFitPanel'
import BackButton from '../BackButton'

// The rulebook a run is held to: the one this bundle carries, which the
// optimizer module is handed (decision 0019 of the suite).
const REGELWERKE = [{ id: KATALOG.catalog.id, name: KATALOG.catalog.title, version: KATALOG.catalog.katalog_version }]

const OPTIMIZE_PREVIEW_SOURCE = 'optimize-preview-source'
const OPTIMIZE_PREVIEW_LAYER  = 'optimize-preview-layer'

const OPTIMIZE_PREVIEW_LAYERS = [{
  sourceId: OPTIMIZE_PREVIEW_SOURCE,
  layer: {
    id: OPTIMIZE_PREVIEW_LAYER, type: 'line',
    paint: {
      'line-color': PALETTE.mapHover,
      'line-width': ZOOM_LINE_WIDTH,
      'line-dasharray': [6, 4],
    },
  },
}]

function previewGeoJSON(elements) {
  const coords = elements.reduce((acc, el, i) => {
    const c = el.renderCoords ?? el.geometry?.coordinates ?? []
    return i === 0 ? [...c] : [...acc, ...c.slice(1)]
  }, [])
  return {
    type: 'FeatureCollection',
    features: [{ type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates: coords } }],
  }
}

// `initialPage`/`onExit` are what the merged splice-and-optimize panel passes:
// it opens the panel straight in a mode and takes the back button back to its
// own menu. Standalone, the panel starts in its own menu as before.
export default function OptimizeTrackPanel({ initialPage = 'menu', onExit, onShowRegelwerk }) {
  const { t, fill } = useI18n()
  const map = useMap()
  const [page, setPage]           = useState(initialPage)    // 'menu' | 'track' | 'element' | 'axis'
  const mode = page
  const [phase, setPhase]         = useState('select')
  const [trackId, setTrackId]     = useState(null)
  const [elementIdx, setElementIdx] = useState(null)    // nur im Element-Modus
  const [label, setLabel]         = useState('')
  // What the run is held to (OptimizeSettings). vMax '' sets no target, open
  // upwards. The rulebook is the one this bundle carries — the module runs on it.
  const [settings, setSettings]   = useState({ corridorCm: 50, grenzwert: 'reg', vMax: '' })
  const setSetting = (key, value) => setSettings(prev => ({ ...prev, [key]: value }))
  const [selectHint, setSelectHint] = useState(null)
  const [running, setRunning]     = useState(false)
  const [run, setRun]             = useState(null)    // { key, result? , error? }
  const abortRef = useRef(null)   // the run under way, to stop it (R10.12)
  const elapsed = useElapsed(running)

  // A result only counts for the parameters it was computed with — derived
  // from the parameter key rather than through an invalidation effect.
  const runKey = [mode, trackId, elementIdx, settings.corridorCm, settings.grenzwert, settings.vMax, phase].join('|')
  const result = run?.key === runKey ? run.result ?? null : null
  const runError = run?.key === runKey ? run.error ?? null : null

  const preview = usePreview(OPTIMIZE_PREVIEW_LAYERS, { resetFilters: [TRACKS_HOVER_LAYER], resetCursor: true })

  // ── Track/element selection ────────────────────────────────────────────────
  useMapPick({
    active: (page === 'track' || page === 'element') && phase === 'select', hover: 'element',
    onPick: ({ trackId: id, elementIndex: elIdx }) => {
      const track = loadTracks().find(tr => tr.id === id)
      if (!track) return
      if (mode === 'element') {
        const el = track.elements?.[elIdx]
        if (!el || el.radius == null) {
          setSelectHint(t('optimize_hint_element_only'))
          return
        }
        setElementIdx(elIdx)
        setLabel(`${trackLabel(track)} · #${elIdx + 1}`)
      } else {
        setElementIdx(null)
        setLabel(trackLabel(track))
      }
      setSelectHint(null)
      setTrackId(track.id)
      setPhase('config')
    },
  })

  // Sync the map preview (external system) with the current result.
  useEffect(() => {
    preview.set(OPTIMIZE_PREVIEW_SOURCE, result ? previewGeoJSON(result.elements) : null)
  }, [result, preview])

  const handleCancel = () => {
    if (map?.current) preview.set(OPTIMIZE_PREVIEW_SOURCE, null)
    setPhase('select')
    setTrackId(null)
    setElementIdx(null)
    setSelectHint(null)
  }

  const backButton = (
    <BackButton onBack={() => { handleCancel(); onExit ? onExit() : setPage('menu') }} />
  )

  const handleRun = () => {
    const track = loadTracks().find(tr => tr.id === trackId)
    if (!track || running) return
    const key = runKey
    setRun(null)
    setRunning(true)
    abortRef.current = new AbortController()
    runOptimizer('optimize', optimizeRequest(track, { ...settings, elementIdx: mode === 'element' ? elementIdx : null }), { signal: abortRef.current.signal })
      .then(res => {
        if (res.error) setRun({ key, error: optimizeErrorText(t, res.error, res.message) })
        else setRun({ key, result: { ...res, elements: reconstructElements(res.elements, track.epsg) } })
      })
      .catch(err => {
        if (err?.name === 'AbortError') { setRun({ key, error: t('run_aborted') }); return }
        setRun({ key, error: moduleErrorText(t, err) })
      })
      .finally(() => setRunning(false))
  }

  const handleCommit = () => {
    const track = loadTracks().find(tr => tr.id === trackId)
    if (!track || !result?.report.some(r => r.changed)) return
    updateTrack(optimizedTrack(track, result))
    handleCancel()
  }

  if (page === 'menu') {
    return (
      <>
        <h2>{t('optimize_track')}</h2>
        <div className="create-element-options">
          <button className="ui-btn ui-btn-option" onClick={() => setPage('track')}>
            <OptimizeTrackModeIcon />
            {t('optimize_mode_track')}
          </button>
          <button className="ui-btn ui-btn-option" onClick={() => setPage('element')}>
            <OptimizeElementModeIcon />
            {t('optimize_mode_element')}
          </button>
          <button className="ui-btn ui-btn-option" onClick={() => setPage('axis')}>
            <AxisFitModeIcon />
            {t('optimize_mode_axis')}
          </button>
        </div>
      </>
    )
  }

  // From measured axis points (AP 12.5): a mode of its own, no track to pick.
  if (page === 'axis') return <AxisFitPanel backButton={backButton} />

  const title = t(mode === 'element' ? 'optimize_mode_element' : 'optimize_mode_track')

  if (phase === 'select') {
    return (
      <>
        {backButton}
        <h2>{title}</h2>
        <p>{t(mode === 'element' ? 'optimize_hint_select_element' : 'optimize_hint_select')}</p>
        {selectHint && (
          <p className="msg-error">{selectHint}</p>
        )}
      </>
    )
  }

  const canCommit = (result?.report.filter(r => r.changed).length ?? 0) > 0
  return (
    <>
      {backButton}
      <h2>{title}</h2>
      <OptimizeSettings label={label} what={mode === 'element' ? t('optimize_mode_element') : 'Track'}
        s={settings} set={setSetting} regelwerke={REGELWERKE} onShowRegelwerk={onShowRegelwerk} />
      <div className="element-form mt-8">
        <button className="ui-btn ui-btn-primary ui-btn-block" onClick={handleRun} disabled={running || !trackId}>{t('optimize_run')}</button>
        {running && (
          <div className="run-status">
            <span>{t('optimize_running')} {fill('run_elapsed', { s: elapsed })}</span>
            <button type="button" className="ui-link" onClick={() => abortRef.current?.abort()}>{t('btn_abort')}</button>
          </div>
        )}
        {runError && <p className="msg-error">{runError}</p>}
        {result && <OptimizeResult result={result} />}
      </div>
      <CommitBar onCommit={handleCommit} onCancel={handleCancel} disabled={!canCommit} />
    </>
  )
}
