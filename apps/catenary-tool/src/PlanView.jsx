import { useEffect, useRef, useState } from 'react'
import { findBasemap } from '@rail-across/basemaps'
import { boundsOf, lngLatAt, nearest, stretch } from './model/net.js'
import { SYSTEM_COLORS } from './model/electrification.js'

/**
 * The plan: the net on the basemap (the suite's, as the other tools have
 * them), every track a line, and on it the ranges under contact wire, each
 * in its system's colour, the chosen one wider. A click picks the range
 * nearest it; while a range is being picked, it gives the track and station
 * nearest it, and the start already picked stands as a dot.
 */

/** How far from a track a click still takes it [px]. */
const PICK_PX = 14
/** The style without a map: the plan on the page's own ground. */
const PLAIN = { version: 8, sources: {}, layers: [{ id: 'ground', type: 'background', paint: { 'background-color': 'rgba(0,0,0,0)' } }] }
export const NO_BASEMAP = 'none'
export const DEFAULT_BASEMAP = 'positron'
const styleOf = (id) => (id === NO_BASEMAP ? PLAIN : findBasemap(id)?.style ?? PLAIN)

const EMPTY = { type: 'FeatureCollection', features: [] }
const line = (coordinates, properties = {}) => ({ type: 'Feature', properties, geometry: { type: 'LineString', coordinates } })

/** What the map's sources hold. */
function sourcesOf(lines, domain, selection, picked) {
  const tracks = { type: 'FeatureCollection', features: [...lines.values()].map(t => line(t.points.map(p => p.ll), { id: t.id })) }
  const ranges = {
    type: 'FeatureCollection',
    features: domain.collections.electrifications.map(e => line(stretch(lines, e.at.trackId, e.at.startStation, e.at.endStation), {
      id: e.id, color: SYSTEM_COLORS[e.system] ?? '#888', selected: e.id === selection ? 1 : 0,
    })).filter(f => f.geometry.coordinates.length >= 2),
  }
  const at = picked && lines.get(picked.trackId) ? lngLatAt(lines.get(picked.trackId).track, picked.station) : null
  const points = at ? { type: 'FeatureCollection', features: [{ type: 'Feature', properties: {}, geometry: { type: 'Point', coordinates: at } }] } : EMPTY
  return { tracks, ranges, points }
}

function addLayers(map) {
  for (const id of ['tracks', 'ranges', 'points']) {
    if (!map.getSource(id)) map.addSource(id, { type: 'geojson', data: EMPTY })
  }
  if (map.getLayer('tracks')) return
  map.addLayer({ id: 'tracks', type: 'line', source: 'tracks', paint: { 'line-color': '#2a2d3a', 'line-width': 2 }, layout: { 'line-cap': 'round' } })
  map.addLayer({
    id: 'ranges', type: 'line', source: 'ranges',
    paint: { 'line-color': ['get', 'color'], 'line-width': ['case', ['==', ['get', 'selected'], 1], 10, 6], 'line-opacity': 0.75 },
    layout: { 'line-cap': 'butt' },
  })
  map.addLayer({ id: 'points', type: 'circle', source: 'points', paint: { 'circle-radius': 6, 'circle-color': '#1c7ed6', 'circle-stroke-color': '#fff', 'circle-stroke-width': 2 } })
}

export default function PlanView({ lines, domain, selection, picking, picked, basemap, fitKey, onSelect, onPick }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const handlers = useRef({})
  handlers.current = { onSelect, onPick, picking, lines, domain }
  const data = useRef(null)
  data.current = sourcesOf(lines, domain, selection, picked)

  const draw = () => {
    const m = map.current
    if (!m || !m.isStyleLoaded()) return
    addLayers(m)
    for (const [id, d] of Object.entries(data.current)) m.getSource(id)?.setData(d)
  }

  // The map, once: north up, no tilt.
  useEffect(() => {
    let gone = false
    import('./maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !box.current) return
      const m = new maplibregl.Map({ container: box.current, style: styleOf(basemap), attributionControl: { compact: true }, dragRotate: false, pitchWithRotate: false })
      m.touchZoomRotate.disableRotation()
      map.current = m
      m.on('style.load', () => { draw(); setReady(true) })
      m.on('click', (e) => {
        const h = handlers.current
        // How many metres a few pixels are here.
        const mpp = 156543.03392 * Math.cos(e.lngLat.lat * Math.PI / 180) / 2 ** m.getZoom()
        const hit = nearest(h.lines, [e.lngLat.lng, e.lngLat.lat], PICK_PX * mpp)
        if (h.picking) { if (hit) h.onPick(hit); return }
        const range = hit && h.domain.collections.electrifications.find(r => r.at.trackId === hit.trackId
          && hit.station >= r.at.startStation - 1 && hit.station <= r.at.endStation + 1)
        h.onSelect(range?.id ?? null)
      })
      m.on('mousemove', (e) => {
        const h = handlers.current
        const mpp = 156543.03392 * Math.cos(e.lngLat.lat * Math.PI / 180) / 2 ** m.getZoom()
        m.getCanvas().style.cursor = nearest(h.lines, [e.lngLat.lng, e.lngLat.lat], PICK_PX * mpp) ? 'pointer' : ''
      })
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Another basemap: the plan's layers again on the new style.
  useEffect(() => { map.current?.setStyle(styleOf(basemap)) }, [basemap])

  // What the domain holds, drawn anew on every change.
  useEffect(() => { if (ready) draw() })

  // The whole net in view: when the tool opens, and when asked.
  useEffect(() => {
    const b = boundsOf(lines)
    if (ready && b) map.current.fitBounds(b, { padding: 60, duration: 0, maxZoom: 17 })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, fitKey])

  return <div className="plan"><div ref={box} className="plan-map" /></div>
}
