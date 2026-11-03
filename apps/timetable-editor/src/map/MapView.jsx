import { useEffect, useMemo, useRef, useState } from 'react'
import { DEFAULT_BASEMAP, findBasemap } from '@rail-across/basemaps'
import { lngLatAt, netGeoJSON, pathGeoJSON } from '@rail-across/running-times'

/**
 * The map of a train: the net on the suite's basemap, its stations, the
 * train's way over it and where it begins, stops, passes and ends.
 */

const EMPTY = { type: 'FeatureCollection', features: [] }
const COLORS = { start: '#1f9e3a', stop: '#d68910', pass: '#ffffff', end: '#c0392b' }

const pointsGeoJSON = (points) => ({
  type: 'FeatureCollection',
  features: points.filter(p => p.lngLat).map(p => ({ type: 'Feature', properties: { kind: p.kind }, geometry: { type: 'Point', coordinates: p.lngLat } })),
})

/** A station on the map: the middle of its tracks' middles, or of its platforms'. */
function stationsGeoJSON(record, stations) {
  return {
    type: 'FeatureCollection',
    features: stations.map(s => {
      const locs = s.tracks.length ? s.tracks.map(tr => ({ trackId: tr.id, station: tr.at ?? tr.length / 2 })) : s.platforms.map(p => ({ trackId: p.trackId, station: p.at }))
      const lls = locs.map(l => lngLatAt(record, l)).filter(Boolean)
      if (!lls.length) return null
      const at = [lls.reduce((a, l) => a + l[0], 0) / lls.length, lls.reduce((a, l) => a + l[1], 0) / lls.length]
      return { type: 'Feature', properties: { name: s.name, kind: s.kind }, geometry: { type: 'Point', coordinates: at } }
    }).filter(Boolean),
  }
}

export default function MapView({ record, net, stations, path, points, t }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)

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
        m.addSource('stations', { type: 'geojson', data: EMPTY })
        m.addLayer({ id: 'net-casing', type: 'line', source: 'net', paint: { 'line-color': '#ffffff', 'line-width': 4.5, 'line-opacity': 0.8 }, layout: { 'line-cap': 'round' } })
        m.addLayer({ id: 'net', type: 'line', source: 'net', paint: { 'line-color': '#2a2d3a', 'line-width': 2.2 }, layout: { 'line-cap': 'round' } })
        m.addLayer({ id: 'path', type: 'line', source: 'path', paint: { 'line-color': '#303383', 'line-width': 7, 'line-opacity': 0.55 }, layout: { 'line-cap': 'round', 'line-join': 'round' } })
        m.addLayer({
          id: 'stations', type: 'circle', source: 'stations',
          paint: { 'circle-radius': ['match', ['get', 'kind'], 'station', 5, 3.5], 'circle-color': '#ffffff', 'circle-stroke-color': '#2a2d3a', 'circle-stroke-width': 2 },
        })
        m.addLayer({
          id: 'points', type: 'circle', source: 'points',
          paint: {
            'circle-radius': ['match', ['get', 'kind'], 'pass', 4, 7], 'circle-color': ['match', ['get', 'kind'], 'start', COLORS.start, 'end', COLORS.end, 'pass', COLORS.pass, COLORS.stop],
            'circle-stroke-color': ['match', ['get', 'kind'], 'pass', COLORS.stop, '#ffffff'], 'circle-stroke-width': 2,
          },
        })
        if (!gone) setReady(true)
      })
      // a station's name where the pointer is on it
      const popup = new maplibregl.Popup({ closeButton: false, closeOnClick: false, offset: 8 })
      m.on('mouseenter', 'stations', (e) => {
        m.getCanvas().style.cursor = 'default'
        const f = e.features?.[0]
        if (f) popup.setLngLat(f.geometry.coordinates).setText(f.properties.name).addTo(m)
      })
      m.on('mouseleave', 'stations', () => popup.remove())
      map.current = m
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
  }, [])

  useEffect(() => { if (ready) map.current.getSource('net').setData(netGeoJSON(net)) }, [ready, net])
  useEffect(() => { if (ready) map.current.getSource('path').setData(path ? pathGeoJSON(net, record, path) : EMPTY) }, [ready, net, record, path])
  useEffect(() => { if (ready) map.current.getSource('points').setData(pointsGeoJSON(points)) }, [ready, points])
  useEffect(() => { if (ready) map.current.getSource('stations').setData(stationsGeoJSON(record, stations)) }, [ready, record, stations])

  // The train's way in view, else the whole net.
  const bounds = useMemo(() => boundsOf(path ? pathGeoJSON(net, record, path) : null) ?? net.bounds, [net, record, path])
  useEffect(() => {
    if (ready && bounds) map.current.fitBounds(bounds, { padding: 40, animate: false, maxZoom: 16 })
  }, [ready, bounds])

  // MapLibre makes its container `position: relative`: it lives inside the box that is placed.
  return <div className="tt-map" role="application" aria-label={t('map')}><div className="tt-map-inner" ref={box} /></div>
}

/** The box around a line's points, or null. */
function boundsOf(geojson) {
  const coords = geojson?.features?.flatMap(f => f.geometry.coordinates) ?? []
  if (coords.length < 2) return null
  const xs = coords.map(c => c[0]), ys = coords.map(c => c[1])
  return [[Math.min(...xs), Math.min(...ys)], [Math.max(...xs), Math.max(...ys)]]
}
