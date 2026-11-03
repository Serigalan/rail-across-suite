import { useEffect, useRef, useState } from 'react'
import { loadTracks } from '../../../storage'
import { elementStations, stationFromClick } from '@rail-across/core/track'
import { wgs84ToUTM } from '@rail-across/core/geometry'
import { connectionPick, isUsableStem } from '@rail-across/core/commands'
import { useI18n } from '../../../locales/i18nContext'
import useMapPick from '../../../map/useMapPick'

/**
 * The two picks a switch connection is laid between — the switch connection
 * and the extended one alike: the first track, then the second, each a click
 * on an element that can carry a turnout, taken at the station clicked
 * (connectionPick). `initial` starts on picks made already (an edited
 * connection, or picks handed over from the other dialog).
 *
 * Returns { phase: 'select_first' | 'select_second' | 'config', picks,
 * pickStatus, reset }.
 */
export default function useConnectionPicks({ initial = null } = {}) {
  const { t } = useI18n()
  const [phase, setPhase] = useState(initial ? 'config' : 'select_first')
  const [picks, setPicks] = useState(initial ?? [])
  const [pickStatus, setPickStatus] = useState(null)    // { msg, error } — selection phases only
  const picksRef = useRef(picks)
  useEffect(() => { picksRef.current = picks }, [picks])

  useMapPick({
    active: phase === 'select_first' || phase === 'select_second', hover: 'element',
    onPick: ({ trackId, elementIndex: elIdx }, e) => {
      const track = loadTracks().find(tr => tr.id === trackId)
      const el    = track?.elements?.[elIdx]
      if (!el) return

      if (!isUsableStem(el)) {
        setPickStatus({ msg: t('scurve_hint_no_bloss'), error: true })
        return
      }

      // The clicked point as a station along the element — along its own curve,
      // whatever kind it is. stationFromClick counts along the whole track.
      const clickUtm = wgs84ToUTM([e.lngLat.lng, e.lngLat.lat], track.epsg)
      const elStart  = elementStations(track)[elIdx]?.start ?? 0
      const along    = Math.min(el.length, Math.max(0, (stationFromClick(track, elIdx, clickUtm) ?? elStart) - elStart))
      const pick     = connectionPick(track, elIdx, along)

      if (phase === 'select_first') {
        setPicks([pick])
        setPickStatus(null)
        setPhase('select_second')
      } else {
        const first = picksRef.current[0]
        if (first && first.trackId === trackId && first.elIdx === elIdx) return
        setPicks([first, pick])
        setPickStatus(null)
        setPhase('config')
      }
    },
  })

  const reset = () => {
    setPhase('select_first')
    setPicks([])
    setPickStatus(null)
  }
  return { phase, picks, pickStatus, reset }
}
