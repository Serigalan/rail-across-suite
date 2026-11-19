import { useMemo, useState } from 'react'
import { alternativeRoutes, heightProfile, lngLatAt, locationAt, marginValue, nearestLocation, netLines, osrdHeightProfile, osrdLngLatAt, osrdPathGeoJSON, osrdRoute, osrdSimulationRequest, osrdViaPlaces, pathGeoJSON, pathLength, placesOf, pointAt, routeThrough, simulate, simulationRequest } from '@rail-across/running-times'
import { comfortsOf } from '@rail-across/vehicles'
import { keepVehicle, keptVehicle } from '../suite.js'
import { useOsrdNet, useOsrdNets } from './useOsrd.js'

/** The vehicle a train starts with: the one chosen last in this browser, else OSRD's regional multiple unit. */
const FIRST_VEHICLE = 'osrd:emu-regional-train'

/** A net with nothing to draw: one of OSRD's while it loads. */
const NO_NET = { lines: new Map(), bounds: null }

/** The ways a train is computed over: the shortest and up to two others (alternativeRoutes). */
const ROUTES = 3

/** The colours of the routes, by their index, on the map and in the list: the shortest's the path's colour of before. */
export const ROUTE_COLORS = ['#303383', '#0e7c86', '#b5528c']

/** The routes' paths on the map as one collection: each a feature with its index and whether it is the one shown. */
function routesGeoJSON(paths, toGeoJSON, shown) {
  return {
    type: 'FeatureCollection',
    features: paths.flatMap((path, i) => toGeoJSON(path).features.map(f => ({ ...f, properties: { route: i, shown: i === shown } }))),
  }
}

/** Of the routes run, the index of the one whose `value` is least, or null. */
function leastOf(routes, value) {
  let best = null
  routes.forEach((r, i) => { const v = value(r); if (v != null && (best == null || v < value(routes[best]))) best = i })
  return best
}

/**
 * Computing the running time of one train on its own, beside the timetables
 * (Fahrzeit rechnen, what the Simulation-Tool was): a train — a vehicle of
 * the suite's catalogue (the Fahrzeug-Editor's), in a traction mode and
 * comfort —, where it starts, stops and ends — signals, platforms or points
 * picked on the map —, the margin it runs with; its path found over the net —
 * the shortest, and on computing up to two other routes (alternativeRoutes) —,
 * and the run over each simulated by rail-across-sim (OSRD's simulation core
 * in Rust): the fastest run, the run with its margin, the times at its stops,
 * the speed and the height over the distance, played on the map, of the route
 * shown (`choice`, at first the fastest). The net is the
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
  // { routes: [{ result, request, properties, path, offsets, chosen, labels, length, via } | { error, path, offsets, chosen, length, via }], key } | { error, key }
  const [runs, setRuns] = useState(null)
  const [choice, setChoice] = useState(0)   // the route shown
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
    setStart(null); setEnd(null); setStops([]); setRuns(null); setPicking(null)
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
  const stale = runs && runs.key !== key
  // The route shown, as the panels and the chart take a run: its result, or its error.
  const routes = runs?.routes ?? null
  const current = routes ? Math.min(choice, routes.length - 1) : 0
  const run = routes ? { ...routes[current], key: runs.key } : runs
  const fastest = routes ? leastOf(routes, r => r.result?.final.total_time) : null
  const shortest = routes ? leastOf(routes, r => r.length) : null
  const choose = (i) => { setChoice(i); setMoment(0) }

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

  /** Each route simulated: its run, or its error. */
  const simulateRoute = async (way, via) => {
    const stopsSpec = [...stops.map((s, i) => ({ offset: way.offsets[i + 1], duration: s.duration ?? 0 })), { offset: way.offsets.at(-1), duration: 0 }]
    const opts = { vehicle, traction: { mode, comfort }, stops: stopsSpec, margin: { value: marginText, distribution } }
    const { request, properties } = netId == null ? simulationRequest(record, way.path, opts) : osrdSimulationRequest(osrd, way.path, opts)
    const found = { path: way.path, offsets: way.offsets, chosen: way.chosen ?? null, length: pathLength(way.path), via }
    try {
      const result = await simulate(request)
      return result.error ? { ...found, error: result.error } : { ...found, result, request, properties, labels: chosen.map(p => p.label) }
    } catch (err) {
      return { ...found, error: { code: err.code ?? 'internal', message: String(err.message ?? err) } }
    }
  }

  const go = async () => {
    if (!route?.path || marginText == null || !vehicle) return
    setBusy(true)
    try {
      // the button says it computes before the routes are searched, which holds the page a moment on a large net
      await new Promise(resolve => setTimeout(resolve, 0))
      const via = [start, ...stops.map(s => s.place), end]
      const ways = alternativeRoutes(
        netId == null ? (weights) => routeThrough(record, via.map(p => p.loc), { weights }) : (weights) => osrdRoute(osrd, via, { weights }),
        { first: route, count: ROUTES, lines: net.lines },
      )
      const names = netId == null ? ways.map(() => null) : osrdViaPlaces(osrd, ways.map(w => w.path))
      const done = []
      for (const [i, way] of ways.entries()) done.push(await simulateRoute(way, names[i]))
      if (!done.some(r => r.result)) setRuns({ error: done[0].error, key })
      else {
        setRuns({ routes: done, key })
        setChoice(leastOf(done, r => r.result?.final.total_time) ?? 0)
        setMoment(0)
        onResult?.()
      }
    } catch (err) {
      setRuns({ error: { code: err.code ?? 'internal', message: String(err.message ?? err) }, key })
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
  // The routes run, while they are the train's; else the shortest way found.
  const shownRoutes = routes && !stale ? routes : null
  const shown = shownRoutes ? current : 0
  // A station of OSRD's is shown at the part the way shown takes, else at its first.
  const takes = shownRoutes ? shownRoutes[shown].chosen : route?.chosen
  const mapPoints = chosen.map((p, i) => p && { kind: i === 0 ? 'start' : i === chosen.length - 1 ? 'end' : 'stop', lngLat: lngLatOf(takes?.[i] ?? p.loc) }).filter(Boolean)
  const pathData = useMemo(
    () => {
      const paths = shownRoutes ? shownRoutes.map(r => r.path) : route?.path ? [route.path] : null
      if (!paths) return null
      if (netId == null) return routesGeoJSON(paths, (path) => pathGeoJSON(net, record, path), shown)
      return osrd ? routesGeoJSON(paths, (path) => osrdPathGeoJSON(osrd, path), shown) : null
    },
    [route, shownRoutes, shown, netId, net, record, osrd],
  )
  const netNotice = netId == null ? (record.tracks?.length ? null : 'empty_net') : loaded.loading ? 'net_loading' : loaded.error ? 'net_failed' : null
  const panelProps = { t, language, record, nets: nets.nets, netId, chooseNet, netLoading: loaded.loading, netError: loaded.error, places, catalogue, entry, vehicle, chooseVehicle, modes, mode, comforts, comfort, setTraction, start, setStart, end, setEnd, stops, setStops, margin, setMargin, marginText, distribution, setDistribution, picking, setPicking, route, run, routes, choice: current, choose, fastest, shortest, stale, busy, go, moment, setMoment, at }
  const viewProps = { t, language, record, net, netNotice, pathData, mapPoints, train, picking, setPicking, pickOnMap, choose: shownRoutes ? choose : null, fitKey, setFitKey, way, ok, profile, at }
  return { panelProps, viewProps }

}

