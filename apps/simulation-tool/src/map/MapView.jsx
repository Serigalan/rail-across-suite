import { useEffect, useRef, useState } from 'react'
import { DEFAULT_BASEMAP, findBasemap } from '@rail-across/basemaps'
import { netGeoJSON, pathGeoJSON } from '../model/places.js'

/**
 * The map: the net on the suite's basemap, the train's path over it, the
 * places it starts, stops and ends at, and — while a run is played — the
 * train. While a place is being picked, a click on the map gives the point
 * and how many metres a few pixels are there, to find the track within reach.
 */

const PICK_PX = 14
const EMPTY = { type: 'FeatureCollection', features: [] }
const COLORS = { start: '#1f9e3a', stop: '#d68910', end: '#c0392b' }

const pointsGeoJSON = (points) => ({
  type: 'FeatureCollection',
  features: points.filter(p => p.lngLat).map(p => ({ type: 'Feature', properties: { kind: p.kind }, geometry: { type: 'Point', coordinates: p.lngLat } })),
})

export default function MapView({ record, net, path, points, train, picking, onPick, fitKey, t }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)
  const handlers = useRef({})
  handlers.current.pick = picking ? onPick : null

  useEffect(() => {
    let gone = false
    import('./maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !box.current) return
      const m = new maplibregl.Map({
        container: box.current, style: findBasemap(DEFAULT_BASEMAP)?.style, attributionControl: { compact: true },
        dragRotate: false, pitchWithRotate: false, touchPitch: false, maxPitch: 0, center: [10, 51], zoom: 5,
      })
      m.touchZoomRotate.disableRotation()
      m.keyboard.disableRotation()
      m.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left')
      m.on('load', () => {
        m.addSource('net', { type: 'geojson', data: EMPTY })
        m.addSource('path', { type: 'geojson', data: EMPTY })
        m.addSource('points', { type: 'geojson', data: EMPTY })
        m.addSource('train', { type: 'geojson', data: EMPTY })
        m.addLayer({ id: 'net-casing', type: 'line', source: 'net', paint: { 'line-color': '#ffffff', 'line-width': 4.5, 'line-opacity': 0.8 }, layout: { 'line-cap': 'round' } })
        m.addLayer({ id: 'net', type: 'line', source: 'net', paint: { 'line-color': '#2a2d3a', 'line-width': 2.2 }, layout: { 'line-cap': 'round' } })
        m.addLayer({ id: 'path', type: 'line', source: 'path', paint: { 'line-color': '#303383', 'line-width': 7, 'line-opacity': 0.55 }, layout: { 'line-cap': 'round', 'line-join': 'round' } })
        m.addLayer({
          id: 'points', type: 'circle', source: 'points',
          paint: { 'circle-radius': 7, 'circle-color': ['match', ['get', 'kind'], 'start', COLORS.start, 'end', COLORS.end, COLORS.stop], 'circle-stroke-color': '#ffffff', 'circle-stroke-width': 2 },
        })
        m.addLayer({ id: 'train', type: 'circle', source: 'train', paint: { 'circle-radius': 8, 'circle-color': '#303383', 'circle-stroke-color': '#ffffff', 'circle-stroke-width': 3 } })
        if (!gone) setReady(true)
      })
      m.on('click', (e) => {
        const pick = handlers.current.pick
        if (!pick) return
        const a = m.unproject([e.point.x, e.point.y]), b = m.unproject([e.point.x + PICK_PX, e.point.y])
        const reach = Math.abs(b.lng - a.lng) * 111320 * Math.cos(a.lat * Math.PI / 180)
        pick([e.lngLat.lng, e.lngLat.lat], reach)
      })
      map.current = m
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
  }, [])

  useEffect(() => { if (ready) map.current.getSource('net').setData(netGeoJSON(net)) }, [ready, net])
  useEffect(() => { if (ready) map.current.getSource('path').setData(path ? pathGeoJSON(net, record, path) : EMPTY) }, [ready, net, record, path])
  useEffect(() => { if (ready) map.current.getSource('points').setData(pointsGeoJSON(points)) }, [ready, points])
  useEffect(() => { if (ready) map.current.getSource('train').setData(train ? pointsGeoJSON([{ kind: 'train', lngLat: train }]) : EMPTY) }, [ready, train])
  useEffect(() => { if (ready) map.current.getCanvas().style.cursor = picking ? 'crosshair' : '' }, [ready, picking])

  // The whole net in view: when it is first there, and again on asking.
  useEffect(() => {
    if (ready && net.bounds) map.current.fitBounds(net.bounds, { padding: 40, animate: false, maxZoom: 17 })
  }, [ready, net, fitKey])

  // MapLibre makes its container `position: relative`: it lives inside the box that is placed.
  return <div className="sim-map" role="application" aria-label={t('map')}><div className="sim-map-inner" ref={box} /></div>
}
