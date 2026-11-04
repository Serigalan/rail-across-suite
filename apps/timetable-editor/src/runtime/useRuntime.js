import { useMemo, useState } from 'react'
import { heightProfile, lngLatAt, locationAt, marginValue, nearestLocation, netLines, placesOf, pointAt, routeThrough, simulate, simulationRequest } from '@rail-across/running-times'
import { comfortsOf } from '@rail-across/vehicles'
import { keepVehicle, keptVehicle } from '../suite.js'

/** The vehicle a train starts with: the one chosen last in this browser, else OSRD's regional multiple unit. */
const FIRST_VEHICLE = 'osrd:emu-regional-train'

/**
 * Computing the running time of one train on its own, beside the timetables
 * (Fahrzeit rechnen, what the Simulation-Tool was): a train — a vehicle of
 * the suite's catalogue (the Fahrzeug-Editor's), in a traction mode and
 * comfort —, where it starts, stops and ends — signals, platforms or points
 * picked on the map —, the margin it runs with; its path found over the net,
 * and the run simulated by rail-across-sim (OSRD's simulation core in Rust):
 * the fastest run, the run with its margin, the times at its stops, the speed
 * and the height over the distance, played on the map. Its state is the
 * editor's, kept while other sections are shown; `onResult` is called when a
 * run has been computed. Its texts are its own (texts.js), `t` here.
 */
export function useRuntime({ record, catalogue, t, language, onResult }) {
  const [vehicleKey, setVehicleKey] = useState(() => keptVehicle() ?? FIRST_VEHICLE)
  const [traction, setTraction] = useState({ mode: null, comfort: 'STANDARD' })
  const [start, setStart] = useState(null)
  const [end, setEnd] = useState(null)
  const [stops, setStops] = useState([])   // [{ key, place, duration }]
  const [margin, setMargin] = useState('5')
  const [distribution, setDistribution] = useState('mareco')
  const [picking, setPicking] = useState(null)   // 'start' | 'end' | 'stop' | null
  const [run, setRun] = useState(null)   // { result, request, properties, path, offsets, labels, key } | { error, key }
  const [busy, setBusy] = useState(false)
  const [moment, setMoment] = useState(0)   // the time played [s]
  const [fitKey, setFitKey] = useState(0)

  const places = useMemo(() => placesOf(record, t), [record, t])
  const net = useMemo(() => netLines(record), [record])
  const trackName = useMemo(() => new Map((record.tracks ?? []).map(tr => [tr.id, tr.name || [tr.lineNumber, tr.trackNumber].filter(Boolean).join(' / ') || tr.id.slice(0, 8)])), [record])
  const chosen = [start, ...stops.map(s => s.place), end]
  const route = useMemo(
    () => (start && end ? routeThrough(record, [start, ...stops.map(s => s.place), end].map(p => p.loc)) : null),
    // the places, not the stops' durations, make the route
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [record, start, end, stops.map(s => s.place.id).join('|')],
  )
  const entry = catalogue.entries.find(e => e.key === vehicleKey) ?? catalogue.entries.find(e => e.key === FIRST_VEHICLE) ?? catalogue.entries[0] ?? null
  const vehicle = entry?.vehicle ?? null
  const modes = Object.keys(vehicle?.effort_curves?.modes ?? {})
  const mode = modes.includes(traction.mode) ? traction.mode : vehicle?.effort_curves?.default_mode
  const comforts = vehicle ? comfortsOf(vehicle, mode) : []
  const comfort = comforts.includes(traction.comfort) ? traction.comfort : 'STANDARD'
  const chooseVehicle = (key) => { setVehicleKey(key); keepVehicle(key); setTraction(tr => ({ ...tr, mode: null })) }
  const marginText = marginValue(margin)
  const key = JSON.stringify([entry?.key, entry?.version, mode, comfort, chosen.map(p => p?.id), stops.map(s => s.duration), marginText, distribution])
  const stale = run && run.key !== key

  const pickOnMap = (lngLat, reach) => {
    const loc = nearestLocation(net, lngLat, reach)
    if (!loc) return
    const place = { id: `map:${loc.trackId}:${loc.station}`, label: t('point_on_track', { track: trackName.get(loc.trackId), station: Math.round(loc.station) }), loc: { ...loc, direction: null } }
    if (picking === 'start') setStart(place)
    else if (picking === 'end') setEnd(place)
    else if (picking === 'stop') setStops(list => [...list, { key: `${Date.now()}`, place, duration: 30 }])
    setPicking(null)
  }

  const go = async () => {
    if (!route?.path || marginText == null || !vehicle) return
    const stopsSpec = [...stops.map((s, i) => ({ offset: route.offsets[i + 1], duration: s.duration ?? 0 })), { offset: route.offsets.at(-1), duration: 0 }]
    const { request, properties } = simulationRequest(record, route.path, { vehicle, traction: { mode, comfort }, stops: stopsSpec, margin: { value: marginText, distribution } })
    setBusy(true)
    try {
      const result = await simulate(request)
      if (result.error) setRun({ error: result.error, key })
      else {
        setRun({ result, request, properties, path: route.path, offsets: route.offsets, labels: chosen.map(p => p.label), key })
        setMoment(0)
        onResult?.()
      }
    } catch (err) {
      setRun({ error: { code: err.code ?? 'internal', message: String(err.message ?? err) }, key })
    } finally {
      setBusy(false)
    }
  }

  const ok = run?.result ? run : null
  const at = ok ? pointAt(ok.result.final, moment) : null
  const train = at ? lngLatAt(record, locationAt(ok.path, at.position)) : null
  // The way the chart shows: the run's, else the one found; its heights along it.
  const way = ok ? { path: ok.path, offsets: ok.offsets, labels: ok.labels } : route?.path ? { path: route.path, offsets: route.offsets, labels: chosen.map(p => p?.label) } : null
  const profile = useMemo(() => (way ? heightProfile(record, way.path) : null), [record, way?.path]) // eslint-disable-line react-hooks/exhaustive-deps
  const mapPoints = chosen.map((p, i) => p && { kind: i === 0 ? 'start' : i === chosen.length - 1 ? 'end' : 'stop', lngLat: lngLatAt(record, p.loc) }).filter(Boolean)
  const panelProps = { t, language, record, places, catalogue, entry, vehicle, chooseVehicle, modes, mode, comforts, comfort, setTraction, start, setStart, end, setEnd, stops, setStops, margin, setMargin, marginText, distribution, setDistribution, picking, setPicking, route, run, stale, busy, go, moment, setMoment, at }
  const viewProps = { t, language, record, net, route, mapPoints, train, picking, setPicking, pickOnMap, fitKey, setFitKey, way, ok, profile, at }
  return { panelProps, viewProps }

}

