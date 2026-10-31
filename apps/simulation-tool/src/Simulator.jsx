import { useMemo, useState } from 'react'
import { IconButton, FitIcon, Message, Rail, RailBrand, RailButton, RailDivider, Shell, WorkingCopyBar, Workspace } from '@rail-across/ui'
import { useTexts } from './i18n.js'
import { ChartIcon, HomeIcon, LogoIcon, SettingsIcon, TrainIcon } from './icons.jsx'
import MapView from './map/MapView.jsx'
import RunChart from './charts/RunChart.jsx'
import TrainPanel from './panels/TrainPanel.jsx'
import ResultPanel from './panels/ResultPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'
import { lngLatAt, nearestLocation, netLines, placesOf } from './model/places.js'
import { locationAt } from './model/path.js'
import { routeThrough } from './model/route.js'
import { marginValue, simulationRequest } from './model/request.js'
import { vehicleOf } from './model/vehicles.js'
import { simulate } from './model/engine.js'
import { pointAt } from './format.js'
import { projectsUrl } from './suite.js'

/**
 * The simulation of one train over the net of a variant, which it only
 * reads: a train — an example vehicle —, where it starts, stops and ends —
 * signals, platforms or points picked on the map —, the margin it runs with;
 * its path found over the net, and the run simulated by rail-across-sim (OSRD's
 * simulation core in Rust): the fastest run, the run with its margin, the
 * times at its stops, the speed and the time over the distance, played on
 * the map.
 */
export default function Simulator({ project, variant, revision, record, local }) {
  const { t, language, setLanguage } = useTexts()
  const [section, setSection] = useState('train')
  const [vehicleId, setVehicleId] = useState('emu')
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
  const vehicle = vehicleOf(vehicleId)
  const marginText = marginValue(margin)
  const key = JSON.stringify([vehicleId, chosen.map(p => p?.id), stops.map(s => s.duration), marginText, distribution])
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
    if (!route?.path || marginText == null) return
    const stopsSpec = [...stops.map((s, i) => ({ offset: route.offsets[i + 1], duration: s.duration ?? 0 })), { offset: route.offsets.at(-1), duration: 0 }]
    const { request, properties } = simulationRequest(record, route.path, { vehicle, stops: stopsSpec, margin: { value: marginText, distribution } })
    setBusy(true)
    try {
      const result = await simulate(request)
      if (result.error) setRun({ error: result.error, key })
      else {
        setRun({ result, request, properties, path: route.path, offsets: route.offsets, labels: chosen.map(p => p.label), key })
        setMoment(0)
        setSection('result')
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
  const mapPoints = chosen.map((p, i) => p && { kind: i === 0 ? 'start' : i === chosen.length - 1 ? 'end' : 'stop', lngLat: lngLatAt(record, p.loc) }).filter(Boolean)
  const panelProps = { t, language, record, places, vehicleId, setVehicleId, start, setStart, end, setEnd, stops, setStops, margin, setMargin, marginText, distribution, setDistribution, picking, setPicking, route, run, stale, busy, go, moment, setMoment, at }

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('suite_sims')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<TrainIcon />} label={t('section_train')} pressed={section === 'train'} onClick={() => setSection('train')} />
          <RailButton icon={<ChartIcon />} label={t('section_result')} pressed={section === 'result'} onClick={() => setSection('result')} />
        </>}
        bottom={<>
          <RailDivider />
          <RailButton icon={<HomeIcon />} label={t('suite_sims')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => setSection('settings')} />
        </>}
      />
      {section === 'train' ? <TrainPanel {...panelProps} />
        : section === 'result' ? <ResultPanel {...panelProps} />
        : <SettingsPanel t={t} language={language} setLanguage={setLanguage} />}
      <Workspace>
        <div className="sim-stack">
          <div className="sim-map-wrap">
            <WorkingCopyBar title={project.title} subtitle={variant.name} revision={revision?.number != null ? t('revision_n', { n: revision.number }) : undefined}
              note={t(local ? 'reads_copy' : 'reads_head')} actions={[{ label: t('back'), onClick: () => location.assign(projectsUrl()) }]} />
            <div className="tool-fit"><IconButton label={t('fit')} icon={<FitIcon />} onClick={() => setFitKey(k => k + 1)} /></div>
            {picking && (
              <div className="tool-mode" role="status">
                <span>{t(`picking_${picking}`)}</span>
                <button type="button" className="ui-btn ui-btn-small" onClick={() => setPicking(null)}>{t('cancel')}</button>
              </div>
            )}
            {!record.tracks?.length && <div className="tool-notice"><Message tone="info">{t('empty_net')}</Message></div>}
            <MapView record={record} net={net} path={route?.path ?? null} points={mapPoints} train={train} picking={picking} onPick={pickOnMap} fitKey={fitKey} t={t} />
          </div>
          {ok && <RunChart run={ok} at={at} t={t} />}
        </div>
      </Workspace>
    </Shell>
  )
}
