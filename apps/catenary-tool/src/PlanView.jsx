import { useEffect, useRef, useState } from 'react'
import { findBasemap } from '@rail-across/basemaps'
import { boundsOf, lngLatAt, nearest, stretch } from './model/net.js'
import { VOLTAGE_COLORS } from './model/electrification.js'

/**
 * The plan: the net on the basemap (the suite's, as the other tools have
 * them), every track a line, and on it the contact wires where they run, each
 * in its voltage's colour, the chosen one wider; the masts as dots beside
 * their track and the section insulators as squares on it. A click picks the
 * mast or insulator nearest it, else the wire over the track there; while
 * something is being picked, it gives the track and station nearest it, and a
 * start already picked stands as a dot.
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
const point = (coordinates, properties = {}) => ({ type: 'Feature', properties, geometry: { type: 'Point', coordinates } })

/** Where an object standing on a track is drawn: [lng, lat], or null. */
const placeOf = (lines, at) => (at?.anchor === 'station' && lines.get(at.trackId) ? lngLatAt(lines.get(at.trackId).track, at.station) : null)

/** What the map's sources hold. */
function sourcesOf(lines, domain, stretches, selection, picked) {
  const tracks = { type: 'FeatureCollection', features: [...lines.values()].map(t => line(t.points.map(p => p.ll), { id: t.id })) }
  const chosen = (collection, id) => (selection?.collection === collection && selection.id === id ? 1 : 0)
  const wires = {
    type: 'FeatureCollection',
    features: stretches.map(s => line(stretch(lines, s.trackId, s.startStation, s.endStation), {
      id: s.contactWireId, color: VOLTAGE_COLORS[s.voltage] ?? '#888', selected: chosen('contactWires', s.contactWireId),
    })).filter(f => f.geometry.coordinates.length >= 2),
  }
  const things = (collection) => domain.collections[collection]
    .map(o => [o, placeOf(lines, o.at)]).filter(([, at]) => at)
    .map(([o, at]) => point(at, { id: o.id, collection, selected: chosen(collection, o.id) }))
  const masts = { type: 'FeatureCollection', features: things('masts') }
  const insulators = { type: 'FeatureCollection', features: things('sectionInsulators') }
  const at = picked && lines.get(picked.trackId) ? lngLatAt(lines.get(picked.trackId).track, picked.station) : null
  const points = at ? { type: 'FeatureCollection', features: [point(at)] } : EMPTY
  return { tracks, wires, masts, insulators, points }
}

function addLayers(map) {
  for (const id of ['tracks', 'wires', 'masts', 'insulators', 'points']) {
    if (!map.getSource(id)) map.addSource(id, { type: 'geojson', data: EMPTY })
  }
  if (map.getLayer('tracks')) return
  map.addLayer({ id: 'tracks', type: 'line', source: 'tracks', paint: { 'line-color': '#2a2d3a', 'line-width': 2 }, layout: { 'line-cap': 'round' } })
  map.addLayer({
    id: 'wires', type: 'line', source: 'wires',
    paint: { 'line-color': ['get', 'color'], 'line-width': ['case', ['==', ['get', 'selected'], 1], 10, 6], 'line-opacity': 0.75 },
    layout: { 'line-cap': 'butt' },
  })
  map.addLayer({
    id: 'masts', type: 'circle', source: 'masts',
    paint: { 'circle-radius': ['case', ['==', ['get', 'selected'], 1], 6, 3.5], 'circle-color': '#2a2d3a', 'circle-stroke-color': '#fff', 'circle-stroke-width': 1 },
  })
  map.addLayer({
    id: 'insulators', type: 'circle', source: 'insulators',
    paint: { 'circle-radius': ['case', ['==', ['get', 'selected'], 1], 8, 6], 'circle-color': '#fff', 'circle-stroke-color': '#2a2d3a', 'circle-stroke-width': 2 },
  })
  map.addLayer({ id: 'points', type: 'circle', source: 'points', paint: { 'circle-radius': 6, 'circle-color': '#1c7ed6', 'circle-stroke-color': '#fff', 'circle-stroke-width': 2 } })
}

export default function PlanView({ lines, domain, stretches, selection, picking, picked, basemap, fitKey, onSelect, onPick }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const handlers = useRef({})
  handlers.current = { onSelect, onPick, picking, lines, stretches }
  const data = useRef(null)
  data.current = sourcesOf(lines, domain, stretches, selection, picked)

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
        const around = [[e.point.x - PICK_PX / 2, e.point.y - PICK_PX / 2], [e.point.x + PICK_PX / 2, e.point.y + PICK_PX / 2]]
        const [thing] = m.queryRenderedFeatures(around, { layers: ['insulators', 'masts'] })
        if (thing) { h.onSelect({ collection: thing.properties.collection, id: thing.properties.id }); return }
        const wire = hit && h.stretches.find(s => s.trackId === hit.trackId && hit.station >= s.startStation - 1 && hit.station <= s.endStation + 1)
        h.onSelect(wire ? { collection: 'contactWires', id: wire.contactWireId } : null)
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
