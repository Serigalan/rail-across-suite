import { useEffect, useRef, useState } from 'react'
import { DEFAULT_BASEMAP, findBasemap } from '@rail-across/basemaps'
import { lngLatAt, locationAt, netGeoJSON, pathLine } from '@rail-across/running-times'
import { extentsOf, sectionsAt, trainAt, trainTitle } from '@rail-across/timetables'

/**
 * The timetable played on the map: the net, and at the moment shown every
 * train under way in its colour — its length along its way, its head, its
 * number by it —, and with `sections` the track sections it occupies drawn
 * in its colour, those its routes hold for it ahead lighter. `focus` { id,
 * n } brings a train into view each time `n` changes.
 */

const EMPTY = { type: 'FeatureCollection', features: [] }

/** The parts of tracks a path covers between two offsets, as a path of its own. */
const piecesOf = (path, from, to) => {
  let at = 0
  const out = []
  for (const p of path) {
    const len = Math.abs(p.end - p.start)
    const a = Math.max(from, at), b = Math.min(to, at + len)
    if (b - a > 0.01) {
      const sign = p.end >= p.start ? 1 : -1
      out.push({ trackId: p.trackId, start: p.start + sign * (a - at), end: p.start + sign * (b - at) })
    }
    at += len
  }
  return out
}

/** What the map shows of the trains at `time`: { bodies, heads, sections } as GeoJSON. */
function scene({ record, net, trains, runs, colors, time, sections }) {
  const bodies = [], heads = [], held = []
  const extents = sections !== 'none' ? extentsOf(record) : null
  const line = (pieces, props) => {
    const coordinates = pathLine(net, record, pieces)
    if (coordinates.length >= 2) return { type: 'Feature', properties: props, geometry: { type: 'LineString', coordinates } }
    return null
  }
  for (const tr of trains) {
    const run = runs.get(tr.id)
    const at = trainAt(run, time)
    if (!at || at.state === 'before' || at.state === 'arrived') continue
    const color = colors.get(tr.id)
    const path = run.route.path
    const length = run.request?.rolling_stock?.length ?? 100
    const body = line(piecesOf(path, Math.max(0, at.offset - length), at.offset), { color })
    if (body) bodies.push(body)
    const ll = lngLatAt(record, locationAt(path, at.offset))
    if (ll) heads.push({ type: 'Feature', properties: { color, label: tr.number || trainTitle(tr), standing: at.state === 'standing' }, geometry: { type: 'Point', coordinates: ll } })
    if (!extents) continue
    const { occupied, reserved } = sectionsAt(record, run, time, { reserved: sections === 'reserved' })
    for (const [ids, kind] of [[occupied, 'occupied'], [reserved, 'reserved']]) {
      for (const id of ids) {
        for (const part of extents.get(id)?.parts ?? []) {
          const f = line([{ trackId: part.trackId, start: part.from, end: part.to }], { color, kind })
          if (f) held.push(f)
        }
      }
    }
  }
  const fc = (features) => ({ type: 'FeatureCollection', features })
  return { bodies: fc(bodies), heads: fc(heads), sections: fc(held) }
}

export default function LiveMap({ record, net, trains, runs, colors, time, sections, focus, t }) {
  const box = useRef(null)
  const map = useRef(null)
  const [ready, setReady] = useState(false)

  useEffect(() => {
    let gone = false
    import('../map/maplibre.js').then(({ default: maplibregl }) => {
      if (gone || !box.current) return
      const m = new maplibregl.Map({
        container: box.current, style: findBasemap(DEFAULT_BASEMAP)?.style, attributionControl: { compact: true },
        dragRotate: false, pitchWithRotate: false, touchPitch: false, maxPitch: 0, center: [10, 51], zoom: 5,
      })
      m.touchZoomRotate.disableRotation()
      m.keyboard.disableRotation()
      m.addControl(new maplibregl.ScaleControl({ unit: 'metric' }), 'bottom-left')
      m.on('load', () => {
        // the basemap's own font for the trains' numbers
        const font = (m.getStyle().layers ?? []).map(l => l.layout?.['text-font']).find(f => Array.isArray(f)) ?? ['Noto Sans Regular']
        for (const id of ['net', 'sections', 'bodies', 'heads']) m.addSource(id, { type: 'geojson', data: EMPTY })
        m.addLayer({ id: 'net-casing', type: 'line', source: 'net', paint: { 'line-color': '#ffffff', 'line-width': 4.5, 'line-opacity': 0.8 }, layout: { 'line-cap': 'round' } })
        m.addLayer({ id: 'net', type: 'line', source: 'net', paint: { 'line-color': '#2a2d3a', 'line-width': 2.2 }, layout: { 'line-cap': 'round' } })
        m.addLayer({
          id: 'sections', type: 'line', source: 'sections',
          paint: { 'line-color': ['get', 'color'], 'line-width': 9, 'line-opacity': ['match', ['get', 'kind'], 'reserved', 0.25, 0.45] },
          layout: { 'line-cap': 'butt' },
        })
        m.addLayer({ id: 'bodies', type: 'line', source: 'bodies', paint: { 'line-color': ['get', 'color'], 'line-width': 6 }, layout: { 'line-cap': 'round', 'line-join': 'round' } })
        m.addLayer({
          id: 'heads', type: 'circle', source: 'heads',
          paint: { 'circle-radius': 6, 'circle-color': ['get', 'color'], 'circle-stroke-color': '#ffffff', 'circle-stroke-width': ['case', ['get', 'standing'], 3.5, 2] },
        })
        m.addLayer({
          id: 'labels', type: 'symbol', source: 'heads',
          layout: { 'text-field': ['get', 'label'], 'text-font': font, 'text-size': 12, 'text-offset': [0, -1.3], 'text-allow-overlap': true },
          paint: { 'text-color': ['get', 'color'], 'text-halo-color': '#ffffff', 'text-halo-width': 1.6 },
        })
        if (!gone) setReady(true)
      })
      map.current = m
    })
    return () => { gone = true; map.current?.remove(); map.current = null }
  }, [])

  useEffect(() => { if (ready) map.current.getSource('net').setData(netGeoJSON(net)) }, [ready, net])
  useEffect(() => {
    if (ready && net.bounds) map.current.fitBounds(net.bounds, { padding: 40, animate: false, maxZoom: 16 })
  }, [ready, net])
  useEffect(() => {
    if (!ready) return
    const s = scene({ record, net, trains, runs, colors, time, sections })
    for (const id of ['bodies', 'heads', 'sections']) map.current.getSource(id).setData(s[id])
  }, [ready, record, net, trains, runs, colors, time, sections])
  // A train asked for brought into view where it is.
  useEffect(() => {
    if (!ready || !focus) return
    const run = runs.get(focus.id)
    const at = trainAt(run, time)
    const ll = at && lngLatAt(record, locationAt(run.route.path, at.offset))
    if (ll) map.current.easeTo({ center: ll, zoom: Math.max(map.current.getZoom(), 13) })
    // only when asked, not as the train runs on
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, focus])

  // MapLibre makes its container `position: relative`: it lives inside the box that is placed.
  return <div className="sim-map" role="application" aria-label={t('map')}><div className="sim-map-inner" ref={box} /></div>
}
