import { useMemo, useState } from 'react'
import { heightProfile, lngLatAt, locationAt, marginValue, nearestLocation, netLines, osrdHeightProfile, osrdLngLatAt, osrdPathGeoJSON, osrdRoute, osrdSimulationRequest, pathGeoJSON, placesOf, pointAt, routeThrough, simulate, simulationRequest } from '@rail-across/running-times'
import { comfortsOf } from '@rail-across/vehicles'
import { keepVehicle, keptVehicle } from '../suite.js'
import { useOsrdNet, useOsrdNets } from './useOsrd.js'

/** The vehicle a train starts with: the one chosen last in this browser, else OSRD's regional multiple unit. */
const FIRST_VEHICLE = 'osrd:emu-regional-train'

/** A net with nothing to draw: one of OSRD's while it loads. */
const NO_NET = { lines: new Map(), bounds: null }

/**
 * Computing the running time of one train on its own, beside the timetables
 * (Fahrzeit rechnen, what the Simulation-Tool was): a train — a vehicle of
 * the suite's catalogue (the Fahrzeug-Editor's), in a traction mode and
 * comfort —, where it starts, stops and ends — signals, platforms or points
 * picked on the map —, the margin it runs with; its path found over the net,
 * and the run simulated by rail-across-sim (OSRD's simulation core in Rust):
 * the fastest run, the run with its margin, the times at its stops, the speed
 * and the height over the distance, played on the map. The net is the
 * variant's, or one of OSRD's the server holds (`netId`, osrd.js of
 * @rail-across/running-times): its operational points the places, its
 * slopes, curves, speed sections and electrification what the train runs
 * with. Its state is the
 * editor's, kept while other sections are shown; `onResult` is called when a
 * run has been computed. Its texts are its own (texts.js), `t` here.
 */
export function useRuntime({ record, catalogue, t, language, onResult }) {
  const [netId, setNetId] = useState(null)   // null: the variant's net; else an OSRD net's id
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

  const nets = useOsrdNets()
  const loaded = useOsrdNet(netId)
  const osrd = loaded.net
  const places = useMemo(() => (netId == null ? placesOf(record, t) : { signals: [], platforms: [], stations: osrd?.places ?? [] }), [record, t, netId, osrd])
  const net = useMemo(() => (netId == null ? netLines(record) : osrd ?? NO_NET), [record, netId, osrd])
  const trackName = useMemo(() => new Map((record.tracks ?? []).map(tr => [tr.id, tr.name || [tr.lineNumber, tr.trackNumber].filter(Boolean).join(' / ') || tr.id.slice(0, 8)])), [record])
  const chosen = [start, ...stops.map(s => s.place), end]
  const route = useMemo(
    () => {
      if (!start || !end) return null
      const via = [start, ...stops.map(s => s.place), end]
      if (netId == null) return routeThrough(record, via.map(p => p.loc))
      return osrd ? osrdRoute(osrd, via) : null
    },
    // the places, not the stops' durations, make the route
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [record, netId, osrd, start, end, stops.map(s => s.place.id).join('|')],
  )
  /** Another net to run over: the places, the train's way and its run belong to the one before. */
  const chooseNet = (id) => {
    if (id === netId) return
    setNetId(id)
    setStart(null); setEnd(null); setStops([]); setRun(null); setPicking(null)
  }
  const lngLatOf = (loc) => (netId == null ? lngLatAt(record, loc) : osrd ? osrdLngLatAt(osrd, loc) : null)
  const entry = catalogue.entries.find(e => e.key === vehicleKey) ?? catalogue.entries.find(e => e.key === FIRST_VEHICLE) ?? catalogue.entries[0] ?? null
  const vehicle = entry?.vehicle ?? null
  const modes = Object.keys(vehicle?.effort_curves?.modes ?? {})
  const mode = modes.includes(traction.mode) ? traction.mode : vehicle?.effort_curves?.default_mode
  const comforts = vehicle ? comfortsOf(vehicle, mode) : []
  const comfort = comforts.includes(traction.comfort) ? traction.comfort : 'STANDARD'
  const chooseVehicle = (key) => { setVehicleKey(key); keepVehicle(key); setTraction(tr => ({ ...tr, mode: null })) }
  const marginText = marginValue(margin)
  const key = JSON.stringify([netId, entry?.key, entry?.version, mode, comfort, chosen.map(p => p?.id), stops.map(s => s.duration), marginText, distribution])
  const stale = run && run.key !== key

  const pickOnMap = (lngLat, reach) => {
    const loc = nearestLocation(net, lngLat, reach)
    if (!loc) return
    const track = netId == null ? trackName.get(loc.trackId) : loc.trackId
    const place = { id: `map:${loc.trackId}:${loc.station}`, label: t('point_on_track', { track, station: Math.round(loc.station) }), loc: { ...loc, direction: null } }
    if (picking === 'start') setStart(place)
    else if (picking === 'end') setEnd(place)
    else if (picking === 'stop') setStops(list => [...list, { key: `${Date.now()}`, place, duration: 30 }])
    setPicking(null)
  }

  const go = async () => {
    if (!route?.path || marginText == null || !vehicle) return
    const stopsSpec = [...stops.map((s, i) => ({ offset: route.offsets[i + 1], duration: s.duration ?? 0 })), { offset: route.offsets.at(-1), duration: 0 }]
    const opts = { vehicle, traction: { mode, comfort }, stops: stopsSpec, margin: { value: marginText, distribution } }
    const { request, properties } = netId == null ? simulationRequest(record, route.path, opts) : osrdSimulationRequest(osrd, route.path, opts)
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
  const train = at ? lngLatOf(locationAt(ok.path, at.position)) : null
  // The way the chart shows: the run's, else the one found; its heights along it.
  const way = ok ? { path: ok.path, offsets: ok.offsets, labels: ok.labels } : route?.path ? { path: route.path, offsets: route.offsets, labels: chosen.map(p => p?.label) } : null
  const profile = useMemo(() => (way ? (netId == null ? heightProfile(record, way.path) : osrdHeightProfile(way.path)) : null), [record, netId, way?.path]) // eslint-disable-line react-hooks/exhaustive-deps
  // A station of OSRD's is shown at the part the way takes, else at its first.
  const mapPoints = chosen.map((p, i) => p && { kind: i === 0 ? 'start' : i === chosen.length - 1 ? 'end' : 'stop', lngLat: lngLatOf(route?.chosen?.[i] ?? p.loc) }).filter(Boolean)
  const pathData = useMemo(
    () => (!route?.path ? null : netId == null ? pathGeoJSON(net, record, route.path) : osrd ? osrdPathGeoJSON(osrd, route.path) : null),
    [route, netId, net, record, osrd],
  )
  const netNotice = netId == null ? (record.tracks?.length ? null : 'empty_net') : loaded.loading ? 'net_loading' : loaded.error ? 'net_failed' : null
  const panelProps = { t, language, record, nets: nets.nets, netId, chooseNet, netLoading: loaded.loading, netError: loaded.error, places, catalogue, entry, vehicle, chooseVehicle, modes, mode, comforts, comfort, setTraction, start, setStart, end, setEnd, stops, setStops, margin, setMargin, marginText, distribution, setDistribution, picking, setPicking, route, run, stale, busy, go, moment, setMoment, at }
  const viewProps = { t, language, record, net, netNotice, pathData, mapPoints, train, picking, setPicking, pickOnMap, fitKey, setFitKey, way, ok, profile, at }
  return { panelProps, viewProps }

}

