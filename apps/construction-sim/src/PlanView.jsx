import { useEffect, useRef, useState } from 'react'
import { findBasemap } from '@rail-across/basemaps'
import { boundsOf } from './model/net.js'
import { STATE_COLORS } from './model/stages.js'

/**
 * The plan: the net on the suite's light basemap at a construction stage —
 * what stands then in black, what comes into service with it in red, what
 * leaves service with the next in orange, the rest faint where it is to be
 * seen —, the connectors and switches of the stage as points. Over it
 * `overlays` — lines of [lng, lat] each with its colour, width and dashes:
 * what a construction scenario closes, a train's ways —; a click on the map
 * goes to `onPick` ([lng, lat]) where it is given.
 */

const STYLE = findBasemap('positron')?.style ?? { version: 8, sources: {}, layers: [] }
const EMPTY = { type: 'FeatureCollection', features: [] }

function addLayers(map) {
  for (const id of ['tracks', 'overlays', 'points']) if (!map.getSource(id)) map.addSource(id, { type: 'geojson', data: EMPTY })
  if (map.getLayer('tracks')) return
  map.addLayer({
    id: 'tracks', type: 'line', source: 'tracks', layout: { 'line-cap': 'round' },
    paint: {
      'line-color': ['get', 'color'], 'line-width': ['case', ['==', ['get', 'state'], 'off'], 1.5, 3.5],
      'line-opacity': ['case', ['==', ['get', 'state'], 'off'], 0.45, 1], 'line-dasharray': ['case', ['==', ['get', 'state'], 'off'], ['literal', [2, 2]], ['literal', [1, 0]]],
    },
  })
  map.addLayer({
    id: 'overlays', type: 'line', source: 'overlays', layout: { 'line-cap': 'round', 'line-join': 'round' },
    paint: { 'line-color': ['get', 'color'], 'line-width': ['get', 'width'], 'line-opacity': 0.9 },
  })
  map.addLayer({
    id: 'overlays-dashed', type: 'line', source: 'overlays', filter: ['==', ['get', 'dashed'], true],
    paint: { 'line-color': '#fff', 'line-width': ['*', ['get', 'width'], 0.4], 'line-dasharray': ['literal', [2, 2]] },
  })
  map.addLayer({ id: 'points', type: 'circle', source: 'points', paint: { 'circle-radius': 5, 'circle-color': ['get', 'color'], 'circle-stroke-color': '#fff', 'circle-stroke-width': 1.5 } })
}

export default function PlanView({ lines, states, points, fitKey, overlays = [], onPick = null }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const data = useRef(null)
  const pick = useRef(onPick)
  useEffect(() => { pick.current = onPick })
  data.current = {
    tracks: { type: 'FeatureCollection', features: [...lines].map(([id, coordinates]) => {
      const state = states.get(id) ?? 'off'
      return { type: 'Feature', properties: { id, state, color: STATE_COLORS[state] }, geometry: { type: 'LineString', coordinates } }
    }) },
    overlays: { type: 'FeatureCollection', features: overlays.filter(o => o.coordinates?.length >= 2).map(o => ({
      type: 'Feature', properties: { color: o.color, width: o.width ?? 5, dashed: Boolean(o.dashed) }, geometry: { type: 'LineString', coordinates: o.coordinates },
    })) },
    points: { type: 'FeatureCollection', features: points.map(p => ({ type: 'Feature', properties: { color: p.color }, geometry: { type: 'Point', coordinates: p.at } })) },
  }
  const draw = () => {
    const m = map.current
    if (!m || !m.isStyleLoaded()) return
    addLayers(m)
    for (const [id, d] of Object.entries(data.current)) m.getSource(id)?.setData(d)
  }

  useEffect(() => {
    let gone = false
    import('./maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !box.current) return
      const m = new maplibregl.Map({ container: box.current, style: STYLE, attributionControl: { compact: true }, dragRotate: false, pitchWithRotate: false })
      m.touchZoomRotate.disableRotation()
      map.current = m
      m.on('style.load', () => { draw(); setReady(true) })
      m.on('click', (e) => pick.current?.([e.lngLat.lng, e.lngLat.lat]))
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
  }, [])
  useEffect(() => { if (ready) draw() })
  useEffect(() => {
    const b = boundsOf(lines)
    if (ready && b) map.current.fitBounds(b, { padding: 60, duration: 0, maxZoom: 17 })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, fitKey])

  return <div className={`plan${onPick ? ' plan-picking' : ''}`}><div ref={box} className="plan-map" /></div>
}
